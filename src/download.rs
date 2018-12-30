// https://www.python.org/ftp/python/
// https://www.python.org/ftp/python/3.7.2/Python-3.7.2.tgz

static DOWNLOAD_URL: &str = "https://www.python.org/ftp/python";

use semver::Version;

use crate::{utils, Result};

pub fn download_source(version: &Version) -> Result<()> {
    unimplemented!()
}
