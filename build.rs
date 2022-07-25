// Build script generates combined & minified html/js files

use std::{fs::File, io::Write};

#[cfg(feature = "minify")]
use minify_html::{Cfg, minify};

#[cfg(feature = "minify")]
const CONFIG: Cfg = Cfg {
    do_not_minify_doctype: true,
    ensure_spec_compliant_unquoted_attribute_values: true,
    keep_closing_tags: false,
    keep_html_and_head_opening_tags: true,
    keep_spaces_between_attributes: true,
    keep_comments: false,
    minify_css: true,
    minify_js: true,
    remove_bangs: true,
    remove_processing_instructions: true,
};

const SCRIPT_TAG: &str = r#"<script type="text/javascript" src="client.js"></script>"#;

fn main() {
    // This could be done in a loop, but since some
    // files need special treatment, we do one at a time.
    // I don't care about efficiency in the build script...

    // prepare data for html file names
    let suf = if cfg!(feature = "tls") {"-tls"} else {""};
    let ext = if cfg!(feature = "minify") {".min.html"} else {".html"};

    // 404 page - just minify
    println!("cargo:rerun-if-changed=page/404.html");
    let html_404 = include_bytes!("page/404.html").as_slice();
    #[cfg(feature = "minify")] let html_404 = &minify(html_404, &CONFIG);
    let mut file = File::create(["target/404", ext].concat()).unwrap();
    file.write_all(html_404).unwrap();

    // client.js - load into html pages
    // can't be minified properly on its own
    println!("cargo:rerun-if-changed=page/client.js");
    let mut js_client = r#"<script type="text/javascript">"#.to_owned();
    js_client.push_str(include_str!("page/client.js"));
    js_client.push_str("</script>");
    #[cfg(feature = "tls")] let js_client = js_client.replace("ws://", "wss://");

    // index - load, minify
    println!("cargo:rerun-if-changed=page/index.html");
    let html_index = include_str!("page/index.html").replace(SCRIPT_TAG, &js_client);
    let html_index_data = html_index.as_bytes();
    #[cfg(feature = "minify")] let html_index_data = &minify(html_index_data, &CONFIG);
    let mut file = File::create(["target/index", suf, ext].concat()).unwrap();
    file.write_all(&html_index_data).unwrap();

    // mobile - load, minify (copy paste of index)
    println!("cargo:rerun-if-changed=page/mobile.html");
    let html_mobile = include_str!("page/mobile.html").replace(SCRIPT_TAG, &js_client);
    let html_mobile_data = html_mobile.as_bytes();
    #[cfg(feature = "minify")] let html_mobile_data = &minify(html_mobile_data, &CONFIG);
    let mut file = File::create(["target/mobile", suf, ext].concat()).unwrap();
    file.write_all(&html_mobile_data).unwrap();
}
