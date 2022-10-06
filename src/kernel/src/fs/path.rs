pub(super) fn decompose<'a>(absolute: &'a str) -> PathComponents<'a> {
    PathComponents(absolute)
}

pub(super) struct PathComponents<'path>(&'path str);

impl<'path> Iterator for PathComponents<'path> {
    type Item = &'path str;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.0.is_empty() {
                break None;
            }

            let end = self.0.find('/').unwrap_or(self.0.len());
            let component = &self.0[..end];

            self.0 = if end == self.0.len() {
                &self.0[end..]
            } else {
                &self.0[(end + 1)..]
            };

            if !component.is_empty() {
                break Some(component);
            }
        }
    }
}
