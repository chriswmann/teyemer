use std::{thread, time};

use clap::Parser;

use rodio::source::{SineWave, Source};
use rodio::{OutputStream, Sink};

#[derive(Debug, Parser)]
struct Args {
    /// Frequency of the first beep in Hz
    #[clap(short, long, default_value = "880")]
    start_freq: f32,
    /// Frequency of the second beep in Hz
    #[clap(short, long, default_value = "1318.51")]
    end_freq: f32,

    /// Duration of the work period (between beeps) in seconds
    #[clap(short, long, default_value = "1180")]
    work_period: u64,

    /// Duration of the rest period in seconds
    #[clap(short, long, default_value = "19")]
    rest_period: u64,

    /// Amplification of the first beep
    #[clap(short, long, default_value = "0.1")]
    start_amplification: f32,

    /// Amplification of the second beep
    #[clap(short, long, default_value = "0.08")]
    end_amplification: f32,
}

/// Teymer is intended to be run in the background while you work, e.g.
/// via cron or systemd. Hence it executes an endless loop.
fn main() {
    let args = Args::parse();
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    // Play the beeps in a continuous loop.
    loop {
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
