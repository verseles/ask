sed -i 'N;s/        if self.visible.load(Ordering::Relaxed) {\n            if io::stdout().is_terminal() {/        if self.visible.load(Ordering::Relaxed) \&\& io::stdout().is_terminal() {/' src/output/spinner.rs
sed -i '/            }/d' src/output/spinner.rs
