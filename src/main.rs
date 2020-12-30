use crate::cni::{get_request, CniRequest};
use anyhow::Result;
use log::info;
use log::LevelFilter;
use log4rs::append::console::{ConsoleAppender, Target};
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::filter::threshold::ThresholdFilter;
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

    exec_cni_command(req.config.ifname.as_str(), &req)?;

    //
    // for (interface, conf) in req.config.interfaces {
    //     exec_cni_command(conf.as_str(), interface.as_str(), &req);
    // }

    Ok(())
}

fn exec_cni_command(ifname: &str, src_req: &CniRequest) -> Result<()> {
    let mut path = PathBuf::new();

    path.push(src_req.path.as_str());
    let subtype = src_req
        .config
        .config
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
    let mut subconf = src_req.config.config.clone();
    subconf.insert(
        String::from("name"),
        serde_json::value::Value::String(src_req.config.name.clone()),
    );

    subconf.insert(
        String::from("cniVersion"),
        serde_json::value::Value::String(src_req.config.cni_version.clone()),
    );

    stdin.write_all(serde_json::to_string(&subconf)?.as_bytes())?;

    let output = handle.wait_with_output()?;
    let raw_output = String::from_utf8(output.stdout)?;

    info!("Got output: {}", raw_output);

    println!("{}", raw_output);
    Ok(())
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
