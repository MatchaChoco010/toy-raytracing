bindgen --with-derive-default --with-derive-eq --allowlist-type="^oidn.*" --allowlist-var="^OIDN.*" --allowlist-function="^oidn.*" --default-enum-style="rust" wrapper.h > src/oidn.rs
