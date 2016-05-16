desc "build"
task :build do
  system "cargo build --target=thumbv7m-none-eabi --features \"mcu_lpc17xx\" --release"
end

desc "build and run test"
task :test do
  system "cargo test --features test -- --nocapture"
end

desc "cleanup"
task :clean do
  system "cargo clean"
end
