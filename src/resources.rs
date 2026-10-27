//! Low-frequency system counters collected away from audio and UI callbacks.
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};
#[derive(Clone, Copy, Default)]
pub struct Sample {
    pub cpu: f32,
    pub used: u64,
}
pub struct Resources {
    fingerprint: Arc<Mutex<Option<String>>>,
    sample: Arc<Mutex<Sample>>,
    stop: Arc<AtomicBool>,
}
impl Resources {
    pub fn new() -> Self {
        let fingerprint = Arc::new(Mutex::new(None));
        let identity = fingerprint.clone();
        let sample = Arc::new(Mutex::new(Sample::default()));
        let output = sample.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let done = stop.clone();
        thread::spawn(move || {
            let mut command = std::process::Command::new("nvidia-smi");
            command.args([
                "--query-gpu=uuid,name,memory.total",
                "--format=csv,noheader,nounits",
            ]);
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                command.creation_flags(0x08000000);
            }
            if let Ok(output) = command.output() {
                if output.status.success() {
                    if let Ok(mut key) = identity.lock() {
                        *key = Some(String::from_utf8_lossy(&output.stdout).trim().to_owned());
                    }
                }
            }
            let mut system = sysinfo::System::new();
            let pid = sysinfo::Pid::from_u32(std::process::id());
            system.refresh_cpu_usage();
            while !done.load(Ordering::Acquire) {
                thread::sleep(Duration::from_secs(1));
                system.refresh_cpu_usage();
                system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
                if let Ok(mut value) = output.lock() {
                    *value = Sample {
                        cpu: system.global_cpu_usage(),
                        used: system.process(pid).map(|p| p.memory()).unwrap_or(0),
                    };
                }
            }
        });
        Self {
            sample,
            stop,
            fingerprint,
        }
    }
    pub fn fingerprint(&self) -> Option<String> {
        self.fingerprint.lock().ok().and_then(|key| key.clone())
    }
    pub fn sample(&self) -> Sample {
        self.sample.lock().map(|s| *s).unwrap_or_default()
    }
}
impl Drop for Resources {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}
