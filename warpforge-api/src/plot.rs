use std::borrow::Borrow;

use derive_more::{Display, FromStr};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::formula::{self, Mount};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum PlotCapsule {
	#[serde(rename = "plot.v1")]
	V1(Plot),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Plot {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub image: Option<formula::Image>,
	pub inputs: IndexMap<LocalLabel, PlotInput>,
	pub steps: IndexMap<StepName, Step>,
	pub outputs: IndexMap<LocalLabel, PlotOutput>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, FromStr, Display)]
pub struct LocalLabel(pub String);

impl Borrow<String> for LocalLabel {
	fn borrow(&self) -> &String {
		&self.0
	}
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, FromStr, Display)]
pub struct StepName(pub String);

impl Borrow<String> for StepName {
	fn borrow(&self) -> &String {
		&self.0
	}
}

#[derive(Clone, Debug, SerializeDisplay, DeserializeFromStr, catverters_derive::Stringoid)]
pub enum PlotInput {
	#[discriminant = "ware"]
	Ware(crate::content::WareID),

	#[discriminant = "mount"]
	Mount(Mount),

	#[discriminant = "literal"]
	Literal(String),

	#[discriminant = "pipe"]
	Pipe(Pipe),

	#[discriminant = "catalog"]
	CatalogRef(crate::catalog::CatalogRef),

	#[discriminant = "ingest"]
	Ingest(Ingest),
}

#[derive(Clone, Debug, SerializeDisplay, DeserializeFromStr, catverters_derive::Stringoid)]
pub enum PlotOutput {
	#[discriminant = "pipe"]
	Pipe(Pipe),
}

#[derive(Clone, Debug, SerializeDisplay, DeserializeFromStr, catverters_derive::Stringoid)]
pub struct Pipe {
	pub step_name: String,
	pub label: LocalLabel,
}

#[derive(Clone, Debug, SerializeDisplay, DeserializeFromStr, catverters_derive::Stringoid)]
pub enum Ingest {
	#[discriminant = "git"]
	Git(GitIngest),
}

#[derive(Clone, Debug, SerializeDisplay, DeserializeFromStr, catverters_derive::Stringoid)]
pub struct GitIngest {
	host_path: String,
	reference: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[allow(clippy::large_enum_variant)]
pub enum Step {
	#[serde(rename = "plot")]
	Plot(Plot),

	#[serde(rename = "protoformula")]
	Protoformula(Protoformula),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Protoformula {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub image: Option<formula::Image>,
	pub inputs: IndexMap<crate::formula::SandboxPort, PlotInput>,
	pub action: crate::formula::Action,
	pub outputs: IndexMap<LocalLabel, crate::formula::GatherDirective>,
}

#[cfg(test)]
mod tests {
	use super::*;

	use crate::test_common::assert_eq_json_roundtrip;
	use expect_test::expect;

	#[test]
	fn test_roundtrip() {
		// https://github.com/warptools/warpsys/blob/bbeb1e6443ed41b27f77db5ed3cc8186a65d1d67/bash/plot.wf
		let expect = expect![[r#"
            {
              "plot.v1": {
                "inputs": {
                  "glibc": "catalog:warpsys.org/bootstrap/glibc:v2.35:amd64",
                  "ld": "catalog:warpsys.org/bootstrap/glibc:v2.35:ld-amd64",
                  "ldshim": "catalog:warpsys.org/bootstrap/ldshim:v1.0:amd64",
                  "make": "catalog:warpsys.org/bootstrap/make:v4.3:amd64",
                  "gcc": "catalog:warpsys.org/bootstrap/gcc:v11.2.0:amd64",
                  "grep": "catalog:warpsys.org/bootstrap/grep:v3.7:amd64",
                  "coreutils": "catalog:warpsys.org/bootstrap/coreutils:v9.1:amd64",
                  "binutils": "catalog:warpsys.org/bootstrap/binutils:v2.38:amd64",
                  "sed": "catalog:warpsys.org/bootstrap/sed:v4.8:amd64",
                  "gawk": "catalog:warpsys.org/bootstrap/gawk:v5.1.1:amd64",
                  "busybox": "catalog:warpsys.org/bootstrap/busybox:v1.35.0:amd64",
                  "src": "catalog:warpsys.org/bash:v5.1.16:src"
                },
                "steps": {
                  "build": {
                    "protoformula": {
                      "inputs": {
                        "/src": "pipe::src",
                        "/lib64": "pipe::ld",
                        "/pkg/glibc": "pipe::glibc",
                        "/pkg/make": "pipe::make",
                        "/pkg/coreutils": "pipe::coreutils",
                        "/pkg/binutils": "pipe::binutils",
                        "/pkg/gcc": "pipe::gcc",
                        "/pkg/sed": "pipe::sed",
                        "/pkg/grep": "pipe::grep",
                        "/pkg/gawk": "pipe::gawk",
                        "/pkg/busybox": "pipe::busybox",
                        "$PATH": "literal:/pkg/make/bin:/pkg/gcc/bin:/pkg/coreutils/bin:/pkg/binutils/bin:/pkg/sed/bin:/pkg/grep/bin:/pkg/gawk/bin:/pkg/busybox/bin",
                        "$CPATH": "literal:/pkg/glibc/include:/pkg/glibc/include/x86_64-linux-gnu"
                      },
                      "action": {
                        "script": {
                          "interpreter": "/pkg/busybox/bin/sh",
                          "contents": [
                            "mkdir -p /bin /tmp /prefix /usr/include/",
                            "ln -s /pkg/glibc/lib /prefix/lib",
                            "ln -s /pkg/glibc/lib /lib",
                            "ln -s /pkg/busybox/bin/sh /bin/sh",
                            "ln -s /pkg/gcc/bin/cpp /lib/cpp",
                            "cd /src/*",
                            "mkdir -v build",
                            "cd build",
                            "export SOURCE_DATE_EPOCH=1262304000",
                            "../configure --prefix=/warpsys-placeholder-prefix LDFLAGS=-Wl,-rpath=XORIGIN/../lib ARFLAGS=rvD",
                            "make",
                            "make DESTDIR=/out install",
                            "sed -i '0,/XORIGIN/{s/XORIGIN/$ORIGIN/}' /out/warpsys-placeholder-prefix/bin/*"
                          ]
                        }
                      },
                      "outputs": {
                        "out": {
                          "from": "/out/warpsys-placeholder-prefix",
                          "packtype": "tar"
                        }
                      }
                    }
                  },
                  "pack": {
                    "protoformula": {
                      "inputs": {
                        "/pack": "pipe:build:out",
                        "/pkg/glibc": "pipe::glibc",
                        "/pkg/ldshim": "pipe::ldshim",
                        "/pkg/busybox": "pipe::busybox",
                        "$PATH": "literal:/pkg/busybox/bin"
                      },
                      "action": {
                        "script": {
                          "interpreter": "/pkg/busybox/bin/sh",
                          "contents": [
                            "mkdir -vp /pack/lib",
                            "mkdir -vp /pack/dynbin",
                            "cp /pkg/glibc/lib/libc.so.6 /pack/lib",
                            "cp /pkg/glibc/lib/libdl.so.2 /pack/lib",
                            "cp /pkg/glibc/lib/libm.so.6 /pack/lib",
                            "mv /pack/bin/bash /pack/dynbin",
                            "cp /pkg/ldshim/ldshim /pack/bin/bash",
                            "cp /pkg/glibc/lib/ld-linux-x86-64.so.2 /pack/lib",
                            "rm -rf /pack/lib/bash /pack/lib/pkgconfig /pack/include /pack/share"
                          ]
                        }
                      },
                      "outputs": {
                        "out": {
                          "from": "/pack",
                          "packtype": "tar"
                        }
                      }
                    }
                  },
                  "test-run": {
                    "protoformula": {
                      "inputs": {
                        "/pkg/bash": "pipe:pack:out"
                      },
                      "action": {
                        "exec": {
                          "command": [
                            "/pkg/bash/bin/bash",
                            "--version"
                          ]
                        }
                      },
                      "outputs": {}
                    }
                  }
                },
                "outputs": {
                  "amd64": "pipe:pack:out"
                }
              }
            }"#]];
		assert_eq_json_roundtrip::<PlotCapsule>(&expect);
	}
}
