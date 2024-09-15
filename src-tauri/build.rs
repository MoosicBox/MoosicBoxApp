use std::io::Write as _;

struct Config {
    web: bool,
    app: bool,
    bundled: bool,
}

impl Config {
    pub fn into_json(self) -> String {
        format!(
            "export const config = {{web:{web},app:{app},bundled:{bundled}}} as const;",
            web = self.web,
            app = self.app,
            bundled = self.bundled,
        )
    }
}

fn main() {
    #[cfg(feature = "bundled")]
    let bundled = true;
    #[cfg(not(feature = "bundled"))]
    let bundled = false;

    let config = Config {
        web: false,
        app: true,
        bundled,
    };

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open("../src/config.ts")
        .unwrap();

    file.write_all(config.into_json().as_bytes()).unwrap();

    tauri_build::build()
}
