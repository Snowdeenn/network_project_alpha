warning: in the working copy of 'Cargo.lock', LF will be replaced by CRLF the next time Git touches it
warning: in the working copy of 'client/Cargo.toml', LF will be replaced by CRLF the next time Git touches it
warning: in the working copy of 'client/src/lib.rs', LF will be replaced by CRLF the next time Git touches it
[1mdiff --git a/Cargo.lock b/Cargo.lock[m
[1mindex 740a1bc..8c7e641 100644[m
[1m--- a/Cargo.lock[m
[1m+++ b/Cargo.lock[m
[36m@@ -542,6 +542,7 @@[m [mdependencies = [[m
  "renet_netcode",[m
  "serde",[m
  "serde_json",[m
[32m+[m[32m "tokio",[m
  "tracing",[m
  "tracing-subscriber",[m
  "utils",[m
[1mdiff --git a/client/Cargo.toml b/client/Cargo.toml[m
[1mindex 33c0ffa..5097463 100644[m
[1m--- a/client/Cargo.toml[m
[1m+++ b/client/Cargo.toml[m
[36m@@ -26,3 +26,4 @@[m [mweave = { git = "https://github.com/Snowdeenn/weave.git" }[m
 egui = "0.36.1"[m
 egui-wgpu = "0.36.1"[m
 egui-winit = "0.36.1"[m
[32m+[m[32mtokio = { version = "1.53.2", features = [ "fs" ] }[m
[1mdiff --git a/client/src/lib.rs b/client/src/lib.rs[m
[1mindex 59be31f..ac53e3a 100644[m
[1m--- a/client/src/lib.rs[m
[1m+++ b/client/src/lib.rs[m
[36m@@ -4,5 +4,6 @@[m [mpub mod graphic_data;[m
 pub mod key;[m
 pub mod rendering;[m
 pub mod ui;[m
[32m+[m[32mpub mod vfx;[m
 [m
 pub use app::App;[m
