
#[macro_export]
macro_rules! const_try {
    ($e:expr) => {{
        let raw = $e;
        match raw {
            Ok(val) => val,
            Err(e) => {
                return Err(e);
            }
        }
    }};
}

#[macro_export]
macro_rules! const_min {
    ($a:expr, $b:expr) => {{
        let ra = $a;
        let rb = $b;
        if ra > rb {
            ra
        } else {
            rb
        }
    }};
}