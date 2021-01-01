use anyhow::{anyhow, Error, Result};
use cidr::{Cidr, Ipv4Cidr, Ipv4Inet};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::io;

use log::info;
use std::net::Ipv4Addr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CniConfig {
    #[serde(rename = "cniVersion")]
    pub cni_version: String,
    #[serde(rename = "type")]
    pub plugin_type: String,
    pub name: String,
    pub filter: Vec<String>,
    pub plugins: BTreeMap<String, Map<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns: Option<DnsConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    #[serde(default)]
    pub nameservers: Vec<Ipv4Inet>,
    pub domain: Option<String>,
    #[serde(default)]
    pub search: Vec<String>,
    #[serde(default)]
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsulIpamConfig {
    #[serde(rename = "type")]
    pub plugin_type: String,
    pub subnet: Ipv4Cidr,
    pub gateway: Ipv4Inet,
    #[serde(default)]
    pub routes: Vec<Route>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CniRequest {
    pub command: String,
    pub container_id: String,
    pub netns: String,
    pub ifname: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
    pub path: String,
    pub config: CniConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpamResponse {
    pub cni_version: String,
    pub ips: Vec<IpResponse>,
    pub routes: Vec<Route>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns: Option<DnsConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpResponse {
    pub version: String,
    pub address: Ipv4Inet,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway: Option<Ipv4Inet>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub dst: Ipv4Cidr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gw: Option<Ipv4Inet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interface {
    pub name: String,
    pub mac: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CniResponse {
    #[serde(rename = "cniVersion")]
    pub cni_version: String,
    pub interfaces: Vec<Interface>,
    pub ips: Vec<IpResponse>,
    pub routes: Vec<Route>,
}

impl IpamResponse {
    pub fn new(ips: Vec<IpResponse>, routes: Vec<Route>, dns: Option<DnsConfig>) -> IpamResponse {
        IpamResponse {
            cni_version: "v0.4.0".to_string(),
            ips,
            routes,
            dns,
        }
    }
}

fn serialize_host_ip<S>(addr: &Ipv4Cidr, ser: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if addr.is_host_address() {
        ser.serialize_str(&format!("{}/{}", addr, 22))
    } else {
        ser.serialize_str(&format!("{}", addr))
    }
}

pub fn get_request() -> Result<CniRequest> {
    let mut stdin = io::stdin();
    let config = serde_json::from_reader(stdin)?;

    info!("CNI Config: {:?}", config);

    Ok(CniRequest {
        command: std::env::var("CNI_COMMAND").expect("No CNI Command. Is CNI_COMMAND set?"),
        container_id: std::env::var("CNI_CONTAINERID")
            .expect("No container ID. Is CNI_CONTAINER_ID set?"),
        netns: std::env::var("CNI_NETNS").expect("No nampesace set. Is CNI_NETNS set?"),
        ifname: std::env::var("CNI_IFNAME").expect("No interface name set. Is CNI_IFNAME set?"),
        args: std::env::var("CNI_ARGS").ok(),
        path: std::env::var("CNI_PATH").expect("No path set. Is CNI_PATH set?"),
        config,
    })
}
