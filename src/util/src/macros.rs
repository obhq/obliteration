/// Expands to the function name of the caller function.
#[macro_export]
macro_rules! function_name {
    () => {{
        // Taken from https://stackoverflow.com/a/40234666/1829232.
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        &name[..name.len() - 3]
    }};
}
