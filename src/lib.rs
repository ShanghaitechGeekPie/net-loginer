use pnet::datalink;
use reqwest::Client;
use std::collections::HashMap;
use std::error::Error;
use std::net::{IpAddr, Ipv4Addr};
use once_cell::sync::Lazy;

static CLIENT: Lazy<Client> = Lazy::new(Client::new);

const VERIFY_CODE_LENGTH: usize = 4;
const CLASSIFICATION_MAX_RETRY: usize = 3;
const NET_AUTH_BASEURL: &str = "https://net-auth.shanghaitech.edu.cn:19008";

pub struct Authenticator {
    user_id: String,
    password: String,
    ip_addresses: Vec<Ipv4Addr>,
}

impl Authenticator {
    pub fn new(user_id: String, password: String) -> Result<Self, Box<dyn Error>> {
        let ip_addresses = datalink::interfaces()
            .into_iter()
            .filter(|iface| iface.is_up() && !iface.is_loopback())
            .flat_map(|iface| iface.ips)
            .filter_map(|ip| match ip.ip() {
                IpAddr::V4(ipv4) if ipv4.octets()[0] == 10 => Some(ipv4),
                _ => None,
            })
            .collect::<Vec<Ipv4Addr>>();

        log::info!("IP addresses: {:?}", ip_addresses);

        Ok(Self {
            user_id,
            password,
            ip_addresses,
        })
    }

    pub async fn perform_login(&mut self) -> Result<(), Box<dyn Error>> {
        for ip_address in &self.ip_addresses {
            log::info!("Logining for IP address: {}", ip_address);

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

            let json_value: serde_json::Value = login_response.json().await?;

            if !json_value["success"].as_bool().unwrap_or(false) {
                return Err(format!("Failed to login, response: {:?}", json_value).into());
            }
        }

        Ok(())
    }
}

impl Authenticator {
    async fn get_verify_code(&self, ip_address: Ipv4Addr) -> Result<String, Box<dyn Error>> {
        for _ in 0..CLASSIFICATION_MAX_RETRY {
            let image_url = format!(
                "{}/portalauth/verificationcode?uaddress={}",
                NET_AUTH_BASEURL, ip_address
            );
    
            let image = CLIENT.get(&image_url).send().await?.bytes().await?;
            let verify_code = ddddocr::ddddocr_classification_old()?.classification(&image)?;
    
            if verify_code.len() == VERIFY_CODE_LENGTH {
                log::info!("Verify code: {}", verify_code);
                return Ok(verify_code);
            }
        }
    
        Err("Failed to get a verify code with length 4 after 3 attempts".into())
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
}
