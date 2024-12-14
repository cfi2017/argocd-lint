use std::str::FromStr;
use std::time::Duration;
use anyhow::Context;
use opentelemetry::KeyValue;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::metrics::reader::{DefaultAggregationSelector, DefaultTemporalitySelector};
use opentelemetry_sdk::{trace, Resource};
use opentelemetry_sdk::trace::{RandomIdGenerator, Sampler};
use serde::Deserialize;
use tracing_error::ErrorLayer;
use tracing_log::LogTracer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Layer;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LoggerConfig {
    level: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TracingConfig {
    logger: LoggerConfig,
}

pub fn init_tracing(cfg: &TracingConfig) -> anyhow::Result<()> {
    if std::env::var_os("RUST_LOG").is_none() {
        // Set `RUST_LOG=todos=debug` to see debug logs,
        // this only shows access logs.
        // this is only unsafe if run concurrently
        unsafe {
            std::env::set_var("RUST_LOG", &cfg.logger.level);
        }
    }
    // log interoperability layer
    LogTracer::init().context("could not initialise log tracer")?;

    // opentelemetry tracing exporter
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(opentelemetry_otlp::new_exporter().tonic().with_endpoint("http://localhost:4317"))
        .with_trace_config(trace::Config::default()
            .with_sampler(Sampler::AlwaysOn)
            .with_id_generator(RandomIdGenerator::default())
            .with_max_events_per_span(64)
            .with_max_attributes_per_span(16)
            .with_max_events_per_span(16)
            .with_resource(Resource::new(vec![KeyValue::new("service.name", "berg-admin")]))
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .context("could not create otel tracing exporter")?;

    // opentelemetry metrics exporter
    let meter = opentelemetry_otlp::new_pipeline()
        .metrics(opentelemetry_sdk::runtime::Tokio)
        .with_exporter(opentelemetry_otlp::new_exporter().tonic().with_endpoint("http://localhost:4317"))
        .with_resource(Resource::new(vec![KeyValue::new("service.name", "berg-admin")]))
        .with_period(Duration::from_secs(3))
        .with_timeout(Duration::from_secs(10))
        .with_aggregation_selector(DefaultAggregationSelector::new())
        .with_temporality_selector(DefaultTemporalitySelector::new())
        .build()
        .context("could not create otel metrics exporter")?;

    // opentelemetry tracing integration layers
    let tracer = tracing_opentelemetry::layer().with_tracer(tracer.tracer("berg-admin"));
    let meter = tracing_opentelemetry::MetricsLayer::new(meter);

    #[cfg(not(debug_assertions))]
    let log_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_filter(tracing_subscriber::filter::EnvFilter::from_default_env());
    #[cfg(debug_assertions)]
    let log_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_filter(tracing_subscriber::filter::EnvFilter::from_default_env());

    // tracing subscriber
    let subscriber = tracing_subscriber::registry()
        .with(log_layer)
        .with(ErrorLayer::default())
        .with(tracer)
        .with(meter);

    tracing::subscriber::set_global_default(subscriber).expect("unable to initialize tracing");
    opentelemetry::global::shutdown_tracer_provider();
    Ok(())
}