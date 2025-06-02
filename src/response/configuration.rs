pub struct Configuration {
  is_proxy:  bool,
  is_raw:    bool,
  is_no_css: bool,
}

impl Configuration {
  pub const fn new() -> Self {
    Self { is_proxy: false, is_raw: false, is_no_css: false }
  }

  pub const fn is_proxy(&self) -> bool { self.is_proxy }

  pub const fn is_raw(&self) -> bool { self.is_raw }

  pub const fn is_no_css(&self) -> bool { self.is_no_css }

  pub const fn set_proxy(&mut self, is_proxy: bool) {
    self.is_proxy = is_proxy;
  }

  pub const fn set_raw(&mut self, is_raw: bool) { self.is_raw = is_raw; }

  pub const fn set_no_css(&mut self, is_no_css: bool) {
    self.is_no_css = is_no_css;
  }
}
