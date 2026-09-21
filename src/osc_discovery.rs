use log::{debug, error, info, warn};
use mdns_sd::{Receiver, ServiceDaemon, ServiceEvent, ServiceInfo};
use std::collections::HashSet;
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio::task::JoinHandle;

const OSCQUERY_SERVICE_TYPE: &str = "_oscjson._tcp.local.";
const VRCHAT_SERVICE_PREFIX: &str = "VRChat-Client-";
const SERVICE_NAME: &str = "PatMe-in-VR";
const SERVICE_HOSTNAME: &str = "patme-in-vr.local.";
const OSCQUERY_ROOT: &str =
    r#"{"FULL_PATH":"/","ACCESS":0,"CONTENTS":{"avatar":{"FULL_PATH":"/avatar","ACCESS":0}}}"#;
const OSCQUERY_AVATAR: &str = r#"{"FULL_PATH":"/avatar","ACCESS":0}"#;

pub(super) struct Discovery {
    mdns: ServiceDaemon,
    http_task: JoinHandle<()>,
    browser_task: JoinHandle<()>,
    vrchat_status: watch::Receiver<bool>,
}

impl Discovery {
    pub(super) async fn new(osc_port: u16) -> io::Result<Self> {
        let listener = TcpListener::bind(("0.0.0.0", 0)).await?;
        let http_port = listener.local_addr()?.port();

        let mdns = ServiceDaemon::new().map_err(mdns_error)?;
        let service = match ServiceInfo::new(
            OSCQUERY_SERVICE_TYPE,
            SERVICE_NAME,
            SERVICE_HOSTNAME,
            (),
            http_port,
            &[] as &[(&str, &str)],
        ) {
            Ok(service) => service.enable_addr_auto(),
            Err(error) => {
                let _ = mdns.shutdown();
                return Err(mdns_error(error));
            }
        };

        if let Err(error) = mdns.register(service) {
            let _ = mdns.shutdown();
            return Err(mdns_error(error));
        }

        let browser = match mdns.browse(OSCQUERY_SERVICE_TYPE) {
            Ok(browser) => browser,
            Err(error) => {
                let _ = mdns.shutdown();
                return Err(mdns_error(error));
            }
        };

        let (vrchat_status_tx, vrchat_status) = watch::channel(false);
        let http_task = tokio::spawn(serve_oscquery(listener, osc_port));
        let browser_task = tokio::spawn(watch_vrchat(browser, vrchat_status_tx));
        info!(
            "OSCQuery service advertised as {} on TCP port {}",
            SERVICE_NAME, http_port
        );

        Ok(Self {
            mdns,
            http_task,
            browser_task,
            vrchat_status,
        })
    }

    pub(super) fn vrchat_status(&self) -> watch::Receiver<bool> {
        self.vrchat_status.clone()
    }
}

impl Drop for Discovery {
    fn drop(&mut self) {
        self.http_task.abort();
        self.browser_task.abort();
        if let Err(error) = self.mdns.stop_browse(OSCQUERY_SERVICE_TYPE) {
            warn!("Failed to stop VRChat discovery: {}", error);
        }
        if let Err(error) = self.mdns.shutdown() {
            warn!("Failed to shut down mDNS: {}", error);
        }
    }
}

fn mdns_error(error: mdns_sd::Error) -> io::Error {
    io::Error::other(format!("mDNS error: {error}"))
}

async fn watch_vrchat(browser: Receiver<ServiceEvent>, status: watch::Sender<bool>) {
    let mut services = HashSet::new();

    while let Ok(event) = browser.recv_async().await {
        match event {
            ServiceEvent::ServiceResolved(service) if is_vrchat_service(service.get_fullname()) => {
                services.insert(service.get_fullname().to_owned());
            }
            ServiceEvent::ServiceRemoved(_, fullname) if is_vrchat_service(&fullname) => {
                services.remove(&fullname);
            }
            ServiceEvent::SearchStopped(_) => services.clear(),
            _ => continue,
        }

        update_status(&status, !services.is_empty());
    }

    update_status(&status, false);
}

fn is_vrchat_service(fullname: &str) -> bool {
    fullname
        .get(..VRCHAT_SERVICE_PREFIX.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(VRCHAT_SERVICE_PREFIX))
}

fn update_status(status: &watch::Sender<bool>, connected: bool) {
    let changed = status.send_if_modified(|current| {
        if *current == connected {
            false
        } else {
            *current = connected;
            true
        }
    });

    if changed {
        info!(
            "VRChat OSCQuery service {}",
            if connected { "detected" } else { "removed" }
        );
    }
}

async fn serve_oscquery(listener: TcpListener, osc_port: u16) {
    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(connection) => connection,
            Err(error) => {
                error!("OSCQuery HTTP accept error: {}", error);
                return;
            }
        };

        tokio::spawn(async move {
            if let Err(error) = serve_oscquery_connection(stream, osc_port).await {
                debug!("OSCQuery HTTP error from {}: {}", peer, error);
            }
        });
    }
}

async fn serve_oscquery_connection(mut stream: TcpStream, osc_port: u16) -> io::Result<()> {
    let mut request = [0_u8; 4096];
    let mut used = 0;

    while used < request.len() {
        let read = stream.read(&mut request[used..]).await?;
        if read == 0 {
            break;
        }
        used += read;
        if request[..used].windows(4).any(|bytes| bytes == b"\r\n\r\n") {
            break;
        }
    }

    let request = String::from_utf8_lossy(&request[..used]);
    let target = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");
    let (status, body) = query_response(target, osc_port);
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).await?;
    stream.shutdown().await
}

fn query_response(target: &str, osc_port: u16) -> (&'static str, String) {
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    if query
        .split('&')
        .filter_map(|part| part.split('=').next())
        .any(|key| key.eq_ignore_ascii_case("HOST_INFO"))
    {
        return (
            "200 OK",
            format!(
                r#"{{"NAME":"{SERVICE_NAME}","OSC_PORT":{osc_port},"OSC_TRANSPORT":"UDP","EXTENSIONS":{{}}}}"#
            ),
        );
    }

    match path {
        "/" => ("200 OK", OSCQUERY_ROOT.to_owned()),
        "/avatar" | "/avatar/" => ("200 OK", OSCQUERY_AVATAR.to_owned()),
        _ => ("404 Not Found", r#"{"ERROR":"Not found"}"#.to_owned()),
    }
}
