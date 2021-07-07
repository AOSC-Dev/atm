use dbus_codegen::GenOpts;
use std::env;
use std::{fs, path::Path};

const PK_DEF: &str = "dbus-xml/org.freedesktop.PackageKit.xml";
const PK_TRANSACTION_DEF: &str = "dbus-xml/org.freedesktop.PackageKit.Transaction.xml";

fn generate_dbus_binding(xmldata: String, name: &str) {
    let options = GenOpts {
        methodtype: None,
        ..Default::default()
    };
    fs::write(
        Path::new(&env::var("OUT_DIR").unwrap()).join(name),
        dbus_codegen::generate(&xmldata, &options)
            .unwrap_or_else(|_| panic!("Failed to generate dbus bindings for {}", name))
            .as_bytes(),
    )
    .unwrap();
}

fn main() {
    let pk = fs::read_to_string(PK_DEF).unwrap();
    let pk_tx = fs::read_to_string(PK_TRANSACTION_DEF).unwrap();
    generate_dbus_binding(pk, "packagekit.rs");
    generate_dbus_binding(pk_tx, "packagekit_tx.rs");
    println!("cargo:rerun-if-changed={}", PK_DEF);
    println!("cargo:rerun-if-changed={}", PK_TRANSACTION_DEF);
}
