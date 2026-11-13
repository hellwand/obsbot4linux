// SPDX-License-Identifier: GPL-3.0-or-later

//! The fake bus of fake mode (5.12): the devices `obsbot4linuxd --fake` sees instead of the
//! sysfs scan, and the test control that plugs, unplugs, sleeps, wakes and fails them.

use crate::contract::{Bus, Camera, CameraError, Found, Lookup, Need, UsbId};
use crate::models::fake::camera::{Dev, FakeCamera, Shape, lock};
use crate::models::fake::{KEYS, REGISTRY, UNKNOWN_PRODUCT, VENDOR, a, b};
use obsbot_proto::{Params, Usb};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

/// One camera operation on a fake device, in the order they happened. Tests read them with
/// [`FakeBus::ops`]; the unsupported device must never have any.
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    Open,
    Identity(Need),
    Read(String),
    Apply(String, Value),
    Act(String, Params),
}

struct Inner {
    devices: Vec<Arc<Mutex<Dev>>>,
    next_devnum: u32,
    next_port: u32,
    serials: BTreeMap<&'static str, u32>,
    delays: BTreeMap<String, Duration>,
}

/// The fake devices of one daemon. All methods are safe to call from any thread.
pub struct FakeBus {
    inner: Mutex<Inner>,
}

fn shape_of(model: &str) -> Option<&'static Shape> {
    match model {
        "fake-a" => Some(a::shape()),
        "fake-b" => Some(b::shape()),
        _ => None,
    }
}

impl FakeBus {
    /// A bus with one device per key, plugged in this order (`fake-a`, `fake-b`, `unknown`;
    /// a key may repeat). Any other key is an error that names it.
    pub fn new(models: &[&str]) -> Result<FakeBus, String> {
        let bus = FakeBus {
            inner: Mutex::new(Inner {
                devices: Vec::new(),
                next_devnum: 1,
                next_port: 1,
                serials: BTreeMap::new(),
                delays: BTreeMap::new(),
            }),
        };
        for model in models {
            bus.plug(model)?;
        }
        Ok(bus)
    }

    fn inner(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn find(&self, id: &str) -> Result<Arc<Mutex<Dev>>, String> {
        self.inner()
            .devices
            .iter()
            .find(|d| lock(d).id_matches(id))
            .cloned()
            .ok_or_else(|| format!("no fake device {id}"))
    }

    /// Plugs a device of `model`: the first unplugged device of that model comes back (same
    /// serial, same USB path, a new device number), else a new one is made. Returns its id
    /// as the daemon will show it once the identity is read.
    pub fn plug(&self, model: &str) -> Result<String, String> {
        if !KEYS.contains(&model) {
            return Err(format!(
                "unknown fake model {model}; known: {}",
                KEYS.join(", ")
            ));
        }
        let mut inner = self.inner();
        let devnum = inner.next_devnum;
        inner.next_devnum += 1;
        let shape = shape_of(model);
        let back = inner.devices.iter().find(|d| {
            let d = lock(d);
            !d.plugged && d.shape.map(|s| s.key) == shape.map(|s| s.key)
        });
        if let Some(dev) = back.cloned() {
            let mut d = lock(&dev);
            d.plugged = true;
            d.devnum = devnum;
            d.asleep = false;
            return Ok(id_of(&d));
        }
        let port = inner.next_port;
        inner.next_port += 1;
        let serial = shape.map(|s| {
            let n = inner.serials.entry(s.key).or_insert(0);
            *n += 1;
            format!("{}{:07}", s.serial_prefix, n)
        });
        let (product, name, interfaces) = match model {
            "fake-a" => (a::PRODUCT, "Fake A", vec!["0e", "0e", "01", "01"]),
            "fake-b" => (b::PRODUCT, "Fake B", vec!["0e", "0e"]),
            _ => (UNKNOWN_PRODUCT, "Unknown Camera", vec!["0e", "0e"]),
        };
        let usb = Usb {
            vendor: format!("{VENDOR:04x}"),
            product: format!("{product:04x}"),
            product_name: Some(name.to_string()),
            bcd_device: "0100".into(),
            path: format!("9-{port}"),
            serial_present: false,
            interfaces: interfaces.into_iter().map(String::from).collect(),
            extension_units: Vec::new(),
        };
        let dev = Dev {
            shape,
            serial,
            usb,
            video_node: format!("/dev/video{}", 100 + 2 * port),
            devnum,
            plugged: true,
            restart_until: None,
            asleep: false,
            fail_next: false,
            delay: inner.delays.get(model).copied().unwrap_or_default(),
            values: shape.map(|s| s.initial.clone()).unwrap_or_default(),
            lagging: BTreeMap::new(),
            ops: Vec::new(),
        };
        let id = id_of(&dev);
        inner.devices.push(Arc::new(Mutex::new(dev)));
        Ok(id)
    }

    /// Unplugs the device with this id (`sn:` or `port:`). The camera keeps its values.
    pub fn unplug(&self, id: &str) -> Result<(), String> {
        let dev = self.find(id)?;
        let mut d = lock(&dev);
        d.plugged = false;
        d.restart_until = None;
        d.lagging.clear();
        Ok(())
    }

    /// Puts the device to sleep, as the camera's own auto-sleep would.
    pub fn sleep(&self, id: &str) -> Result<(), String> {
        lock(&*self.find(id)?).asleep = true;
        Ok(())
    }

    pub fn wake(&self, id: &str) -> Result<(), String> {
        lock(&*self.find(id)?).asleep = false;
        Ok(())
    }

    /// The next camera operation on this device fails with `NoAnswer`, an identity attempt
    /// included. The flag survives an unplug, so a re-plug can start with a failed identity.
    pub fn fail_next(&self, id: &str) -> Result<(), String> {
        lock(&*self.find(id)?).fail_next = true;
        Ok(())
    }

    /// Every camera operation on devices of `model`, plugged now or later, takes this long
    /// (cut by the operation's context). Not on the test-control socket: for in-process tests.
    pub fn set_delay(&self, model: &str, delay: Duration) {
        let mut inner = self.inner();
        inner.delays.insert(model.to_string(), delay);
        for dev in &inner.devices {
            let mut d = lock(dev);
            if d.shape.is_some_and(|s| s.key == model) {
                d.delay = delay;
            }
        }
    }

    /// The operations sent to the device with this id, oldest first.
    pub fn ops(&self, id: &str) -> Vec<Op> {
        self.find(id)
            .map(|d| lock(&d).ops.clone())
            .unwrap_or_default()
    }

    /// One line of the test-control socket (`{"plug":"fake-a"}`, `{"unplug":"<id>"}`,
    /// `{"sleep":"<id>"}`, `{"wake":"<id>"}`, `{"fail_next":"<id>"}`) → the reply line:
    /// `{"ok":"<id>"}` for `plug`, `{"ok":true}` for the others, `{"error":"<message>"}`.
    pub fn handle_line(&self, line: &str) -> String {
        let reply = match serde_json::from_str::<Value>(line) {
            Err(e) => Err(format!("not JSON: {e}")),
            Ok(Value::Object(map)) if map.len() == 1 => {
                let (verb, arg) = map.into_iter().next().expect("one entry");
                match (verb.as_str(), arg.as_str()) {
                    ("plug", Some(m)) => self.plug(m).map(|id| json!(id)),
                    ("unplug", Some(id)) => self.unplug(id).map(|_| json!(true)),
                    ("sleep", Some(id)) => self.sleep(id).map(|_| json!(true)),
                    ("wake", Some(id)) => self.wake(id).map(|_| json!(true)),
                    ("fail_next", Some(id)) => self.fail_next(id).map(|_| json!(true)),
                    _ => Err(format!("unknown control {verb}")),
                }
            }
            Ok(_) => Err("a control line is one object with one key".into()),
        };
        match reply {
            Ok(v) => json!({ "ok": v }).to_string(),
            Err(e) => json!({ "error": e }).to_string(),
        }
    }
}

fn id_of(d: &Dev) -> String {
    match &d.serial {
        Some(serial) => obsbot_proto::sn_id(serial),
        None => obsbot_proto::port_id(&d.usb.path),
    }
}

impl Bus for FakeBus {
    fn scan(&self) -> Vec<Found> {
        let mut inner = self.inner();
        let now = Instant::now();
        let mut found = Vec::new();
        let devices = inner.devices.clone();
        for dev in devices {
            let mut d = lock(&dev);
            if !d.plugged {
                continue;
            }
            if let Some(until) = d.restart_until {
                if now < until {
                    continue;
                }
                d.restart_until = None;
                d.devnum = inner.next_devnum;
                inner.next_devnum += 1;
            }
            let model = match UsbId::from_usb(&d.usb).map(|id| REGISTRY.lookup(id)) {
                Some(Lookup::Model(entry)) => Some(entry),
                Some(Lookup::Unsupported) => None,
                Some(Lookup::NotListed) | None => continue,
            };
            found.push(Found {
                usb: d.usb.clone(),
                devnum: d.devnum,
                video_node: Some(d.video_node.clone()),
                model,
            });
        }
        found
    }

    fn open(&self, found: &Found) -> Result<Box<dyn Camera>, CameraError> {
        let inner = self.inner();
        for dev in &inner.devices {
            let mut d = lock(dev);
            if d.usb.path == found.usb.path
                && d.plugged
                && d.restart_until.is_none()
                && d.devnum == found.devnum
            {
                let Some(shape) = d.shape else {
                    return Err(CameraError::Io("not a supported fake model".into()));
                };
                d.ops.push(Op::Open);
                let devnum = d.devnum;
                drop(d);
                return Ok(Box::new(FakeCamera {
                    shape,
                    dev: dev.clone(),
                    devnum,
                }));
            }
        }
        Err(CameraError::Gone)
    }
}
