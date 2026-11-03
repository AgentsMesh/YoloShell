fn main() -> anyhow::Result<()> {
    yolo_web_cli::run_from(std::env::args_os())
}
