desc "build"
task :build do
  system "cargo build --target=thumbv7m-none-eabi --release"
end

desc "cleanup"
task :clean do
  system "cargo clean"
end
