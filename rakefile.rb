desc "build"
task :build do
  sh "cargo build --target=thumbv7m-none-eabi --release"
  sh "srec_cat ./target/thumbv7m-none-eabi/release/rustyfridge -binary -o ./target/thumbv7m-none-eabi/release/rustyfridge.hex -intel"
end

desc "cleanup"
task :clean do
  sh "cargo clean"
end
