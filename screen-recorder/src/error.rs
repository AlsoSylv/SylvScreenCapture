#[derive(Debug)]
pub enum Error {
    Win32(windows::core::Error),
    Detour(retour::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Win32(e) => e.fmt(f),
            Error::Detour(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

impl From<retour::Error> for Error {
    fn from(value: retour::Error) -> Self {
        Error::Detour(value)
    }
}

impl From<windows::core::Error> for Error {
    fn from(value: windows::core::Error) -> Self {
        Error::Win32(value)
    }
}
