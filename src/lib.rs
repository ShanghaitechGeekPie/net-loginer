use get_if_addrs::{get_if_addrs, IfAddr};
use once_cell::sync::Lazy;
use reqwest::Client;
use std::collections::HashMap;
use std::error::Error;
use std::net::Ipv4Addr;
use std::str::FromStr;
use thiserror::Error;

static CLIENT: Lazy<Client> = Lazy::new(Client::new);
const NET_AUTH_BASEURL: &str = "https://net-auth.shanghaitech.edu.cn:19008";

#[derive(Debug, PartialEq)]
pub enum AuthResult {
    InvalidVerifyCode,
    UserNotFound,
    InvalidPassword(i64, u64),
    UserLocked(u64),
    Success,
}

#[derive(Debug, Error)]
pub enum AuthParseError {
    #[error("Response missing field: {0}")]
    FieldNotFound(String),
    #[error("Unsupported error code: {0}")]
    UnsupportedErrorCode(String),
    #[error("Failed to parse field: {0}, origin value: {1}")]
    FieldParseError(String, serde_json::Value),
}

pub struct Authenticator {
    user_id: String,
    password: String,
    ip_addresses: Vec<Ipv4Addr>,
}

impl Authenticator {
    pub fn new(user_id: String, password: String) -> Result<Self, Box<dyn Error>> {
        let ip_addresses: Vec<Ipv4Addr> = get_if_addrs()?
            .into_iter()
            .filter_map(|if_addr| match if_addr.addr {
                IfAddr::V4(ipv4) if ipv4.ip.octets()[0] == 10 => Some(ipv4.ip),
                _ => None,
            })
            .collect();

        log::info!("IP addresses: {:?}", ip_addresses);

        Ok(Self {
            user_id,
            password,
            ip_addresses,
        })
    }

    pub async fn perform_login(&self) -> Result<(), Box<dyn Error>> {
        for ip_address in &self.ip_addresses {
            log::info!("Logining for IP address: {}", ip_address);
            self.login_for_ip(ip_address).await?;
        }
        Ok(())
    }
}

impl Authenticator {
    async fn get_verify_code(&self, ip_address: Ipv4Addr) -> Result<String, Box<dyn Error>> {
        let image_url = format!(
            "{}/portalauth/verificationcode?uaddress={}",
            NET_AUTH_BASEURL, ip_address
        );

        let image = CLIENT.get(&image_url).send().await?.bytes().await?;
        let verify_code = ddddocr::ddddocr_classification_old()?.classification(&image)?;

        log::info!("Verify code: {}", verify_code);
        return Ok(verify_code);
    }

    async fn get_page_params(
        &self,
        ip_address: Ipv4Addr,
    ) -> Result<(String, String), Box<dyn Error>> {
        let verify_url = format!(
            "{}/portal?uaddress={}&ac-ip=0",
            NET_AUTH_BASEURL, ip_address
        );

        let redirected_verify = CLIENT.get(&verify_url).send().await?;
        let redirected_url = redirected_verify.url();
        let query_params: HashMap<_, _> = redirected_url.query_pairs().into_owned().collect();

        let push_page_id = query_params
            .get("pushPageId")
            .ok_or("Cannot find pushPageId in query parameters")?
            .to_string();
        let ssid = query_params
            .get("ssid")
            .ok_or("Cannot find ssid in query parameters")?
            .to_string();

        log::info!("Get pushPageId: {:?}", push_page_id);
        log::info!("Get ssid: {:?}", ssid);

        Ok((push_page_id, ssid))
    }

    fn parse_auth_result(
        &self,
        json_value: &serde_json::Value,
    ) -> Result<AuthResult, Box<dyn Error>> {
        if json_value["success"]
            .as_bool()
            .ok_or(AuthParseError::FieldNotFound("success".to_string()))?
        {
            return Ok(AuthResult::Success);
        }

        let error_code = json_value["errorcode"]
            .as_str()
            .ok_or(AuthParseError::FieldNotFound("errorcode".to_string()))?
            .parse::<u64>()?;

        let response_data = &json_value["data"];

        fn parse_field<T: FromStr>(
            response_data: &serde_json::Value,
            field: &str,
        ) -> Result<T, Box<dyn Error>>
        where
            <T as FromStr>::Err: Error + 'static,
        {
            let parse_result = response_data[field]
                .as_str()
                .ok_or(AuthParseError::FieldNotFound(field.to_string()))?
                .parse::<T>()?;

            Ok(parse_result)
        }

        match error_code {
            3010 => Ok(AuthResult::InvalidVerifyCode),
            10505 => {
                let remain_lock_time = parse_field(response_data, "remainLockTime")?;
                Ok(AuthResult::UserLocked(remain_lock_time))
            }
            10503 => {
                if response_data.is_null() {
                    Ok(AuthResult::UserNotFound)
                } else {
                    let remain_times = parse_field(response_data, "remainTimes")?;
                    let lock_time = parse_field(response_data, "lockTime")?;
                    Ok(AuthResult::InvalidPassword(remain_times, lock_time))
                }
            }
            _ => Err(AuthParseError::UnsupportedErrorCode(error_code.to_string()).into()),
        }
    }

    async fn login_for_ip(&self, ip_address: &Ipv4Addr) -> Result<(), Box<dyn Error>> {
        loop {
            let ((push_page_id, ssid), verify_code) = tokio::try_join!(
                self.get_page_params(*ip_address),
                self.get_verify_code(*ip_address)
            )?;

            let login_data = serde_json::json!({
                "userName": self.user_id,
                "userPass": self.password,
                "uaddress": ip_address,
                "validCode": verify_code,
                "pushPageId": push_page_id,
                "ssid": ssid,
                "agreed": "1",
                "authType": "1",
            });

            let login_response = CLIENT
                .post(&format!("{}/portalauth/login", NET_AUTH_BASEURL))
                .form(&login_data)
                .send()
                .await?;

            let json_value = login_response.json().await?;
            let auth_result = self.parse_auth_result(&json_value)?;

            match auth_result {
                AuthResult::Success => {
                    log::info!("Login successful for IP address: {}", ip_address);
                    break;
                }
                AuthResult::InvalidVerifyCode => {
                    log::warn!("Invalid verify code: {}, retrying...", verify_code)
                }
                AuthResult::UserNotFound => {
                    log::warn!("User not found: {}", self.user_id);
                    return Err("User not found".into());
                }
                AuthResult::UserLocked(remain_lock_time) => {
                    log::warn!("You are locked. Remaining lock time: {}", remain_lock_time);
                    return Err("User locked".into());
                }
                AuthResult::InvalidPassword(remain_times, lock_time) => {
                    log::warn!(
                        "Invalid password. Enter the wrong password {} more times and you will be locked out for {} minute",
                        remain_times,
                        lock_time
                    );
                    return Err("Invalid password".into());
                }
            }
        }
        Ok(())
    }
}
