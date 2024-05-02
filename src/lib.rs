use pnet::datalink;
use reqwest::Client;
use std::collections::HashMap;
use std::error::Error;

const NET_AUTH_BASEURL: &str = "https://net-auth.shanghaitech.edu.cn:19008";

pub struct Authenticator {
    user_id: String,
    password: String,
    ip_address: String,
    client: Client,
}

impl Authenticator {
    pub fn new(
        user_id: String,
        password: String,
        interface: String,
    ) -> Result<Self, Box<dyn Error>> {
        let ip_address = {
            let interface = datalink::interfaces()
                .into_iter()
                .find(|iface| iface.name == interface)
                .ok_or(format!("Cannot find interface: {}", interface))?;

            interface.ips.first().unwrap().ip().to_string()
        };

        log::info!("IP address of {}: {}", interface, ip_address);

        Ok(Self {
            user_id,
            password,
            ip_address,
            client: Client::new(),
        })
    }

    async fn get_verify_code(&self) -> Result<String, Box<dyn Error>> {
        let image_url = format!(
            "{}/portalauth/verificationcode?uaddress={}",
            NET_AUTH_BASEURL, self.ip_address
        );

        let image = self.client.get(&image_url).send().await?.bytes().await?;
        let verify_code = ddddocr::ddddocr_classification_old()?.classification(&image)?;
        log::info!("Verify code: {}", verify_code);

        Ok(verify_code)
    }

    async fn get_page_params(&self) -> Result<(String, String), Box<dyn Error>> {
        let verify_url = format!(
            "{}/portal?uaddress={}&ac-ip=0",
            NET_AUTH_BASEURL, self.ip_address
        );
        log::info!("Verify URL: {}", verify_url);

        let redirected_verify = self.client.get(&verify_url).send().await?;
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

    pub async fn perform_login(&mut self) -> Result<(), Box<dyn Error>> {
        let (page_params, verify_code) =
            tokio::join!(self.get_page_params(), self.get_verify_code());

        let (push_page_id, ssid) = page_params?;

        let login_data = serde_json::json!({
            "userName": self.user_id,
            "userPass": self.password,
            "uaddress": self.ip_address,
            "validCode": verify_code?,
            "pushPageId": push_page_id,
            "ssid": ssid,
            "agreed": "1",
            "authType": "1",
        });

        let login_response = self
            .client
            .post(&format!("{}/portalauth/login", NET_AUTH_BASEURL))
            .form(&login_data)
            .send()
            .await?;

        let json_value: serde_json::Value = login_response.json().await?;

        if !json_value["success"].as_bool().unwrap_or(false) {
            return Err(format!("Failed to login, response: {:?}", json_value).into());
        }

        Ok(())
    }
}
