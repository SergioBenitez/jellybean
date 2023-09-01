#[macro_export]
macro_rules! cmd {
    ($bin:expr) => (cmd!($bin,));
    ($cwd:expr => $bin:expr) => (cmd!($cwd => $bin,));
    ($cwd:expr => $bin:expr, $($token:tt)*) => {{
        let mut cmd = std::process::Command::new($bin);
        cmd.current_dir($cwd);
        cmd!(@arg[cmd] $($token)*);
        cmd.spawn().and_then(|mut p| match p.wait() {
            Ok(e) if e.success() => Ok(()),
            Ok(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())),
            Err(e) => Err(e)
        })
    }};
    ($bin:expr, $($token:tt)*) => {{
        let mut cmd = std::process::Command::new($bin);
        cmd!(@arg[cmd] $($token)*);
        cmd.spawn().and_then(|mut p| match p.wait() {
            Ok(e) if e.success() => Ok(()),
            Ok(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())),
            Err(e) => Err(e)
        })
    }};
    (@arg[$cmd:expr] #$args:expr) => ($cmd.args($args));
    (@arg[$cmd:expr] #$args:expr, $($token:tt)*) => {
        cmd!(@arg[$cmd] #$args);
        cmd!(@arg[$cmd] $($token)*);
    };
    (@arg[$cmd:expr] $arg:expr) => ($cmd.arg($arg));
    (@arg[$cmd:expr] $arg:expr, $($token:tt)*) => {
        cmd!(@arg[$cmd] $arg);
        cmd!(@arg[$cmd] $($token)*);
    };
    (@arg[$cmd:expr] $(,)*) => ();
}

#[macro_export] #[cfg(windows)] macro_rules! slash { () => { r#"\"# } }
#[macro_export] #[cfg(not(windows))] macro_rules! slash { () => {   "/"  } }

#[macro_export]
macro_rules! path {
    ($root:expr $(, $path:literal)*) => {
        std::path::Path::new(concat!($root, $($crate::slash!(), $path),*))
   }
}

#[macro_export]
macro_rules! crate_path {
    ($($path:literal),*) => {
        path!(std::env!("CARGO_MANIFEST_DIR"), $($path),*)
   }
}
