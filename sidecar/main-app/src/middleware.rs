use actix_web::dev::{Service, Transform, ServiceRequest, ServiceResponse};
use actix_web::Error;
use futures::future::{ok, Ready};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Instant;
use std::net::UdpSocket;
use log::{info, error};

pub struct PrometheusMetricsMiddleware;

impl PrometheusMetricsMiddleware {
    pub fn new() -> Self {
        PrometheusMetricsMiddleware
    }
}

// Define the service struct
pub struct PrometheusMetricsMiddlewareService<S> {
    service: S,
}

impl<S> Transform<S, ServiceRequest> for PrometheusMetricsMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse, Error = Error> + 'static,
{
    type Response = ServiceResponse;
    type Error = Error;
    type Transform = PrometheusMetricsMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(PrometheusMetricsMiddlewareService { service })
    }
}

impl<S, B> Service<ServiceRequest> for PrometheusMetricsMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start = Instant::now();
        let path = req.path().to_string();
        let method = req.method().to_string();

        info!("Processing request: {} {}", method, path);

        let fut = self.service.call(req);
        
        Box::pin(async move {
            let res = fut.await?;
            let duration = start.elapsed().as_secs_f64();
            let status = res.status().as_u16().to_string();

            // Send metrics via UDP
            if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
                let metric = format!(
                    r#"request{{path="{}",method="{}",status={}}} {}"#,
                    path, method, status, duration
                );
                
                info!("Sending metric: {}", metric);
                
                // Send to the sidecar's UDP port (9092)
                match socket.send_to(metric.as_bytes(), "sidecar:9092") {
                    Ok(_) => info!("Metric sent successfully"),
                    Err(e) => error!("Failed to send metric: {}", e),
                }
            } else {
                error!("Failed to bind UDP socket");
            }

            Ok(res)
        })
    }
}