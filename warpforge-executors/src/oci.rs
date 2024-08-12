use serde_json::json;

pub trait ToOCIMount {
	fn to_oci_mount(&self) -> serde_json::Value;
}

impl ToOCIMount for crate::MountSpec {
	fn to_oci_mount(&self) -> serde_json::Value {
		json!({
			"destination": self.destination,
			"type": self.kind,
			"source": self.source,
			"options": self.options,
		})
	}
}

// Below, we have the defaults and templates for OCI container config.
// That means "runc" and "gvisor" in practice.  (There may be others, but this is what we test and know.)
//
// These are we have essentially static values.
// However, a factory function returning a consistent thing turns out easier than actually making them static.
// (Rust has a "lazy_static" feature, but it's a crate rather than core, and doesn't seem essential here.)
//
// We use json values because that's what they are when they get sent to the subprocesses.
// We also (plan to -- future work) let users amend these values by a simple JSON Patch API.
// So, overall, KISS means "just treat it like JSON all the way through".
//
// (Yes, there is a crate for OCI spec stuff: https://github.com/containers/oci-spec-rs --
// but at the moment, I don't see how using it would provide significant value.
// And I'm outright suspicious of some of it -- such as https://docs.rs/oci-spec/0.6.2/src/oci_spec/runtime/miscellaneous.rs.html#161 -- what's that "gid=5" special case doing there??)

fn oci_spec_default_caps() -> serde_json::Value {
	json!(["CAP_AUDIT_WRITE", "CAP_KILL", "CAP_NET_BIND_SERVICE"])
}

// Values not included in this, but definitely needed, include:
//    "process":{"args": ["sh"]}
pub fn oci_spec_base() -> serde_json::Value {
	json!({
		"ociVersion": "1.0.0",
		"process": {
			"terminal": false,
			"user": {"uid": 0, "gid": 0},
			"env": [
				"PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
			],
			"cwd": "/",
			"capabilities": {
				"bounding": oci_spec_default_caps(),
				"effective": oci_spec_default_caps(),
				"inheritable": oci_spec_default_caps(),
				"permitted": oci_spec_default_caps(),
			},
			"rlimits": [{
				"type": "RLIMIT_NOFILE",
				"hard": 1024,
				"soft": 1024,
			}]
		},
		"root": {
			"path": "REPLACEME",
			"readonly": true, // frequently overriden to false.
		},
		"hostname": "forge",
		"mounts": [
			{
				"destination": "/proc",
				"type": "proc",
				"source": "proc"
			},
			{
				"destination": "/dev",
				"type": "tmpfs",
				"source": "tmpfs"
			}
		],
		"linux": {
			// Some touchy bits in this area.
			//  1. Rootless operation with `runc` requires UID and GID mappings.
			//     With `gvisor`, those mappings are ignored.
			//  2. Rootless operation with `runc` requires a "user" ns.
			//     With `gvisor`, that ns causes the gofer process to fail to launch.
			//  3. Whether a "network" ns shows up here or not doesn't entirely say whether network will be had.
			//     With `gvisor`, host or none network is specified at the CLI.
			// So, these values below are the common ground,
			//  but more must be stacked up before this value is usable.
			"namespaces": [
				{"type": "pid"},
				{"type": "ipc"},
				{"type": "uts"},
				{"type": "mount"},
			]
		}
	})
}
