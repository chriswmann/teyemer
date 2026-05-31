use std::{thread, time};

use clap::Parser;

use rodio::source::{SineWave, Source};
use rodio::{OutputStreamBuilder, Sink};
use tracing::debug;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
struct Args {
    /// Frequency of the first beep in Hz
    #[clap(long, default_value = "880")]
    start_freq: f32,
    /// Frequency of the second beep in Hz
    #[clap(long, default_value = "1318.51")]
    end_freq: f32,

    /// Duration of the work period (between beeps) in seconds
    #[clap(long, default_value = "1180")]
    work_period: u64,

    /// Duration of the rest period in seconds
    #[clap(long, default_value = "19")]
    rest_period: u64,

    /// Amplification of the first beep
    #[clap(long, default_value = "0.06")]
    start_amplification: f32,

    /// Amplification of the second beep
    #[clap(long, default_value = "0.04")]
    end_amplification: f32,
}

/// Teyemer is intended to be run in the background while you work, e.g.
/// via cron or systemd. Hence it executes an endless loop.
fn main() {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Play the beeps in a continuous loop.
    loop {
        debug!("teyemer running; first beep in {work_period} seconds");
        // Open the stream in the loop so that a transient loss of device doesn't result in
        // permanent loss of audio. It is a small overhead over a 20-minute cycle.
        let stream_handle = OutputStreamBuilder::open_default_stream()
            .expect("Should be able to open default stream");
        let sink = Sink::connect_new(stream_handle.mixer());
        thread::sleep(time::Duration::from_secs(args.work_period));
        let source = SineWave::new(args.start_freq)
            .take_duration(time::Duration::from_secs_f32(0.5))
            .amplify(args.start_amplification);
        sink.append(source);
        sink.sleep_until_end();
        thread::sleep(time::Duration::from_secs(args.rest_period));
        let source = SineWave::new(args.end_freq)
            .take_duration(time::Duration::from_secs_f32(0.5))
            .amplify(args.end_amplification);
        sink.append(source);
        sink.sleep_until_end();
    }
}
