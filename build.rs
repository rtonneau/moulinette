fn main() {
    #[cfg(feature = "gui")]
    slint_build::compile_with_config(
        "ui/main.slint",
        slint_build::CompilerConfiguration::new()
            .with_style("fluent".into())
            .with_debug_info(std::env::var("DEBUG").as_deref() == Ok("true")),
    )
    .expect("compile Slint interface");
}
