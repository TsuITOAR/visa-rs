use visa_sys as vs;


#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Session(vs::ViSession);

impl Drop for Session {
    fn drop(&mut self) {
        unsafe {
            vs::viClose(self.0);
        }
    }
}