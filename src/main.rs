use crate::cni::{get_request, CniRequest, CniResponse, Interface, IpResponse, Route};
use anyhow::Result;
use log::info;
use log::LevelFilter;
use log4rs::append::console::{ConsoleAppender, Target};
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::filter::threshold::ThresholdFilter;
use serde_json::{Map, Value};
use std::env::join_paths;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

mod cni;

fn main() -> Result<()> {
    init_logging();
    info!("Running CNI Multi Plugin");

    let req = get_request()?;

    info!("Handling request: {:?}", req);

    let mut interfaces: Vec<Interface> = Vec::new();
    let mut ips: Vec<IpResponse> = Vec::new();
    let mut routes: Vec<Route> = Vec::new();
    for (ifname, config) in req.config.plugins.clone() {
        if let Some(resp) = exec_cni_command(ifname.as_str(), config, &req)? {
            interfaces.extend(resp.interfaces);
            if !req.config.filter.contains(&ifname) {
                ips.extend(resp.ips);
            }
            routes.extend(resp.routes);
        }
    }

    if req.command == "DEL" {
        info!("Not sending response to DEL");
        return Ok(());
    }

    let resp = CniResponse {
        cni_version: String::from("0.4.0"),
        interfaces,
        ips,
        routes,
    };

    info!("Sending resp {:?}", resp);

    println!("{}", serde_json::to_string(&resp)?);

    Ok(())
}

fn exec_cni_command(
    ifname: &str,
    config: Map<String, Value>,
    src_req: &CniRequest,
) -> Result<Option<CniResponse>> {
    let mut path = PathBuf::new();

    path.push(src_req.path.as_str());
    let subtype = config
        .get("type")
        .expect("No plugin type!")
        .as_str()
        .expect("Plugin type was not string");

    path.push(subtype);

    let cmd_path = path.as_path();

    info!("Executing command: {:?}", cmd_path);

    let mut handle = Command::new(path)
        .env("CNI_IFNAME", ifname)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let stdin = handle.stdin.as_mut().expect("Could not open child stdin");

    // Write the sub-config
    let mut subconf = config.clone();
    subconf.insert(
        String::from("name"),
        serde_json::value::Value::String(src_req.config.name.clone()),
    );

    subconf.insert(
        String::from("cniVersion"),
        serde_json::value::Value::String(src_req.config.cni_version.clone()),
    );

    let subreq = serde_json::to_string(&subconf)?;
    info!("Sending sub req: {}", subreq.as_str());

    stdin.write_all(subreq.as_bytes())?;

    let output = handle.wait_with_output()?;

    // For deletes, jsut pretend all is well!
    if src_req.command == "DEL" {
        return Ok(None);
    }

    let raw_output = String::from_utf8(output.stdout)?;
    if !output.status.success() {
        println!("{}", raw_output);
        anyhow::bail!("Error in plugin");
    }

    info!("Got raw output: {:?}", raw_output);

    let resp: CniResponse = serde_json::from_str(raw_output.as_str())?;
    info!("Got output: {:?}", resp);

    Ok(Some(resp))
}

fn init_logging() {
    let stdout = ConsoleAppender::builder().target(Target::Stderr).build();

    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} - {l} - {L} - {m}{n}")))
        .build("/tmp/log/cni-multi.log")
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("stderr", Box::new(stdout)))
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(LevelFilter::Info)))
                .build("logfile", Box::new(logfile)),
        )
        .build(
            Root::builder()
                .appender("logfile")
                .build(LevelFilter::Trace),
        )
        .unwrap();

    let _handle = log4rs::init_config(config).unwrap();
}
