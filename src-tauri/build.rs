fn main() {
  // Export directory for ts-rs generated bindings
  std::env::set_var("TS_RS_EXPORT_DIR", "../src/src/bindings");
  tauri_build::build()
}
