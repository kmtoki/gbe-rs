pub struct Logger<A> {
    pub logging: bool,
    pub buffer: Vec<A>,
    cap: usize,
    pos: usize,
    overflow: bool,
}

impl<A: Default + Clone> Logger<A> {
    pub fn new(cap: usize) -> Logger<A> {
        let mut v = Vec::with_capacity(cap);
        v.resize(cap, Default::default());
        Logger {
            logging: true,
            buffer: v,
            cap: cap,
            pos: 0,
            overflow: false,
        }
    }

    pub fn write(&mut self, a: A) {
        if self.logging {
            if self.pos + 1 < self.cap {
                self.pos += 1;
            } else {
                self.pos = 0;
                self.overflow = true;
            }

            self.buffer[self.pos] = a;
        }
    }

    pub fn read(&self) -> &A {
        &self.buffer[self.pos]
    }

    pub fn reads(&self, n: usize) -> &'_ [A] {
        if self.overflow {}

        &self.buffer[self.pos - n..=self.pos]
    }
}
