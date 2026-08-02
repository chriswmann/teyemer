use std::io::{Read as _, Write as _};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

use clap::{Parser, Subcommand};

use rodio::source::{SineWave, Source};
use rodio::{DeviceSinkBuilder, Player};
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

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Mute the beeps of a running teyemer for the given number of minutes
    Mute { minutes: u64 },
}

/// Teyemer is intended to be run in the background while you work, e.g.
/// via cron or systemd. Hence it executes an endless loop.
fn main() {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Use the same executable to send the mute command to a running teyemer process
    if let Some(Command::Mute { minutes }) = args.command {
        send_mute(minutes);
        return;
    }

    let mute_until: Arc<Mutex<Option<SystemTime>>> = Arc::new(Mutex::new(None));
    spawn_listener(Arc::clone(&mute_until));

    // Play the beeps in a continuous loop.
    loop {
        let work_period = args.work_period;
        let rest_period = args.rest_period;
        let start_freq = args.start_freq;
        let end_freq = args.end_freq;
        let start_amplification = args.start_amplification;
        let end_amplification = args.end_amplification;
        debug!("teyemer running; first beep in {work_period} seconds");
        // Open the stream in the loop so that a transient loss of device doesn't result in
        // permanent loss of audio. It is a small overhead over a 20-minute cycle.
        let stream_handle =
            DeviceSinkBuilder::open_default_sink().expect("Should be able to open default stream");
        let player = Player::connect_new(stream_handle.mixer());
        thread::sleep(Duration::from_secs(work_period));

        // A beep pair is muted as a unit, and silence overrides in both directions:
        // muted at the start silences both beeps, even if the mute expires during
        // the rest period and a mute arriving during the rest period silences
        // the end beep too.
        let muted_at_start = is_muted(&mute_until);
        if !muted_at_start {
            beep(&player, start_freq, start_amplification);
        }

        thread::sleep(Duration::from_secs(rest_period));
        if !(muted_at_start || is_muted(&mute_until)) {
            beep(&player, end_freq, end_amplification);
        }
    }
}

fn socket_path() -> PathBuf {
    let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(dir).join("teyemer.sock")
}

fn send_mute(minutes: u64) {
    let mut stream = UnixStream::connect(socket_path())
        .expect("should be able to connect to teyemer socket; is teyemer running?");
    writeln!(stream, "mute {minutes}").expect("should be able to send command via UnixStream");
}

fn spawn_listener(mute_until: Arc<Mutex<Option<SystemTime>>>) {
    let path = socket_path();
    // A previous run leaves the socket file behind; bind fails with AddrInUse otherwise.
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).expect("should be able to bind control socket");
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buf = String::new();
            if stream.read_to_string(&mut buf).is_err() {
                continue;
            }
            if let Some(mins) = buf
                .trim()
                .strip_prefix("mute ")
                .and_then(|s| s.parse::<u64>().ok())
            {
                *mute_until.lock().unwrap() =
                    Some(SystemTime::now() + Duration::from_secs(mins * 60));
                debug!("muted for {mins} minutes");
            }
        }
    });
}

fn is_muted(mute_until: &Mutex<Option<SystemTime>>) -> bool {
    mute_until
        .lock()
        .unwrap()
        .is_some_and(|t| SystemTime::now() < t)
}

fn beep(player: &Player, freq: f32, amplification: f32) {
    let source = SineWave::new(freq)
        .take_duration(Duration::from_secs_f32(0.5))
        .amplify(amplification);
    player.append(source);
    player.sleep_until_end();
}
