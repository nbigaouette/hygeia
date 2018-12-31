use std::{
    fs::{create_dir_all, File},
    io::{BufWriter, Read, Write},
    path::{Path, PathBuf},
};

use failure::format_err;
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, error, info, warn};
use regex::Regex;
use semver::Version;
use url::Url;

use crate::{utils, Result};

pub fn download_from_url<P: AsRef<Path>>(url: &Url, download_to: P) -> Result<()> {
    let download_to = download_to.as_ref();

    if !utils::path_exists(&download_to) {
        debug!("Directory {:?} does not exists. Creating.", download_to);
        create_dir_all(&download_to)?;
    }

    let filename = url
        .path_segments()
        .ok_or_else(|| format_err!("Could not extract filename from url"))?
        .last()
        .ok_or_else(|| format_err!("Could not get last segment from url path"))?
        .to_string();

    let mut file_path = PathBuf::new();
    file_path.push(download_to);
    file_path.push(&filename);

    if file_path.exists() {
        info!("File {} already downloaded. Skipping.", filename);
        Ok(())
    } else {
        info!("Downloading {}...", url);

        let mut resp = reqwest::get(url.as_str())?;

        if resp.status().is_success() {
            let headers = resp.headers().clone();
            let ct_len = match headers
                .get(reqwest::header::CONTENT_LENGTH)
                .map(|ct_len| ct_len.clone())
            {
                Some(ct_len) => {
                    let ct_len: u64 = ct_len.to_str()?.parse()?;
                    debug!("Downloading {} bytes...", ct_len);
                    Some(ct_len)
                }
                None => {
                    warn!("Could not find out file size");
                    None
                }
            };

            let chunk_size = match ct_len {
                Some(x) => x / 99_u64,
                None => 1024_u64, // default chunk size
            } as usize;

            let bar = create_progress_bar(&filename, ct_len);

            let mut out = BufWriter::new(File::create(file_path)?);

            loop {
                let mut buffer = vec![0; chunk_size];
                let bcount = resp.read(&mut buffer[..])?;
                buffer.truncate(bcount);
                if buffer.is_empty() {
                    break;
                } else {
                    out.write_all(&mut buffer)?;
                    bar.inc(bcount as u64);
                }
            }

            bar.finish();

            Ok(())
        } else {
            error!("Failed to download {}: {:?}", resp.url(), resp.status());
            let res = resp.error_for_status();
            res.map(|_| ())
                .map_err(|e| format_err!("Failed to download file: {:?}", e))
        }
    }
}

pub fn download_source(version: &Version) -> Result<()> {
    let url = build_url(&version)?;
    let download_dir = utils::pycors_home()?.join("downloads");
    download_from_url(&url, download_dir)
}

fn build_url(version: &Version) -> Result<Url> {
    let main_version = format!("{}.{}", version.major, version.minor);
    let version_path = if version.patch == 0 {
        main_version.clone()
    } else {
        format!("{}.{}", main_version, version.patch)
    };
    let version_file = format!("{}", version).replace("-", "");

    let to_download = Url::parse(&format!(
        "https://www.python.org/ftp/python/{}/Python-{}.tgz",
        version_path, version_file
    ))?;

    Ok(to_download)
}

fn create_progress_bar(msg: &str, length: Option<u64>) -> ProgressBar {
    let bar = match length {
        Some(len) => ProgressBar::new(len),
        None => ProgressBar::new_spinner(),
    };

    bar.set_message(msg);
    match length.is_some() {
        true => bar
            .set_style(ProgressStyle::default_bar()
                .template("{msg} {spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} eta: {eta}")
                .progress_chars("=> ")),
        false => bar.set_style(ProgressStyle::default_spinner()),
    };

    bar
}

pub fn find_all_python_versions() -> Result<Vec<Version>> {
    let index_html = reqwest::get("https://www.python.org/ftp/python/")?.text()?;

    parse_index_html(&index_html)
}

fn parse_index_html(index_html: &str) -> Result<Vec<Version>> {
    let re = Regex::new(r#"(?x)<a \s+ href="(?P<version>\d+[\d\.]+)/">"#)?;

    re.captures_iter(index_html)
        .map(|caps| {
            let v = &caps["version"];
            // Add a `.0` for versions missing a patch number (f.e. `2.7`)
            let dots = v.chars().filter(|c| *c == '.').count();
            let v = if dots == 1 {
                format!("{}.0", v)
            } else {
                v.to_string()
            };
            Version::parse(&v).map_err(|e| format_err!("Failed to parse version: {}", e))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_url_from_version_372() {
        let version = Version::parse("3.7.2").unwrap();

        let url = build_url(&version).unwrap();

        assert_eq!(
            url,
            Url::parse("https://www.python.org/ftp/python/3.7.2/Python-3.7.2.tgz").unwrap()
        );
    }

    #[test]
    fn build_url_from_version_372rc1() {
        let version = Version::parse("3.7.2-rc1").unwrap();

        let url = build_url(&version).unwrap();

        assert_eq!(
            url,
            Url::parse("https://www.python.org/ftp/python/3.7.2/Python-3.7.2rc1.tgz").unwrap()
        );
    }

    #[test]
    fn parse_html() {
        let index_html = r#"<html>
                        <head><title>Index of /ftp/python/</title></head>
                        <body bgcolor="white">
                        <h1>Index of /ftp/python/</h1><hr><pre><a href="../">../</a>
                        <a href="2.0/">2.0/</a>                                               23-Aug-2001 14:13                   -
                        <a href="2.0.1/">2.0.1/</a>                                             06-Aug-2001 02:14                   -
                        <a href="2.1/">2.1/</a>                                               06-Aug-2001 02:14                   -
                        <a href="2.1.1/">2.1.1/</a>                                             16-Aug-2001 17:59                   -
                        <a href="2.1.2/">2.1.2/</a>                                             08-Feb-2002 19:46                   -
                        <a href="2.1.3/">2.1.3/</a>                                             23-Apr-2002 08:38                   -
                        <a href="2.2/">2.2/</a>                                               23-Apr-2002 08:38                   -
                        <a href="2.2.1/">2.2.1/</a>                                             23-Apr-2002 08:36                   -
                        <a href="2.2.2/">2.2.2/</a>                                             28-Feb-2014 07:07                   -
                        <a href="2.2.3/">2.2.3/</a>                                             13-Jun-2003 06:30                   -
                        <a href="2.3/">2.3/</a>                                               28-Feb-2014 07:07                   -
                        <a href="2.3.1/">2.3.1/</a>                                             20-Mar-2014 21:57                   -
                        <a href="2.3.2/">2.3.2/</a>                                             20-Mar-2014 21:55                   -
                        <a href="2.3.3/">2.3.3/</a>                                             20-Mar-2014 21:57                   -
                        <a href="2.3.4/">2.3.4/</a>                                             20-Mar-2014 21:57                   -
                        <a href="2.3.5/">2.3.5/</a>                                             20-Mar-2014 21:58                   -
                        <a href="2.3.6/">2.3.6/</a>                                             01-Nov-2006 07:25                   -
                        <a href="2.3.7/">2.3.7/</a>                                             20-Mar-2014 21:58                   -
                        <a href="2.4/">2.4/</a>                                               20-Mar-2014 21:57                   -
                        <a href="2.4.1/">2.4.1/</a>                                             20-Mar-2014 21:57                   -
                        <a href="2.4.2/">2.4.2/</a>                                             20-Mar-2014 21:59                   -
                        <a href="2.4.3/">2.4.3/</a>                                             20-Mar-2014 22:00                   -
                        <a href="2.4.4/">2.4.4/</a>                                             20-Oct-2006 07:39                   -
                        <a href="2.4.5/">2.4.5/</a>                                             20-Mar-2014 21:56                   -
                        <a href="2.4.6/">2.4.6/</a>                                             20-Mar-2014 21:59                   -
                        <a href="2.5/">2.5/</a>                                               20-Mar-2014 22:00                   -
                        <a href="2.5.1/">2.5.1/</a>                                             20-Mar-2014 21:56                   -
                        <a href="2.5.2/">2.5.2/</a>                                             20-Mar-2014 21:59                   -
                        <a href="2.5.3/">2.5.3/</a>                                             20-Mar-2014 21:59                   -
                        <a href="2.5.4/">2.5.4/</a>                                             20-Mar-2014 21:57                   -
                        <a href="2.5.5/">2.5.5/</a>                                             20-Mar-2014 21:59                   -
                        <a href="2.5.6/">2.5.6/</a>                                             20-Mar-2014 21:58                   -
                        <a href="2.6/">2.6/</a>                                               20-Mar-2014 21:59                   -
                        <a href="2.6.1/">2.6.1/</a>                                             20-Mar-2014 21:57                   -
                        <a href="2.6.2/">2.6.2/</a>                                             20-Mar-2014 21:57                   -
                        <a href="2.6.3/">2.6.3/</a>                                             20-Mar-2014 21:55                   -
                        <a href="2.6.4/">2.6.4/</a>                                             20-Mar-2014 22:00                   -
                        <a href="2.6.5/">2.6.5/</a>                                             20-Mar-2014 22:00                   -
                        <a href="2.6.6/">2.6.6/</a>                                             20-Mar-2014 21:58                   -
                        <a href="2.6.7/">2.6.7/</a>                                             20-Mar-2014 21:59                   -
                        <a href="2.6.8/">2.6.8/</a>                                             20-Mar-2014 21:57                   -
                        <a href="2.6.9/">2.6.9/</a>                                             20-Mar-2014 21:58                   -
                        <a href="2.7/">2.7/</a>                                               20-Mar-2014 21:58                   -
                        <a href="2.7.1/">2.7.1/</a>                                             20-Mar-2014 21:59                   -
                        <a href="2.7.10/">2.7.10/</a>                                            23-May-2015 22:27                   -
                        <a href="2.7.11/">2.7.11/</a>                                            05-Dec-2015 21:32                   -
                        <a href="2.7.12/">2.7.12/</a>                                            27-Jun-2016 15:52                   -
                        <a href="2.7.13/">2.7.13/</a>                                            17-Dec-2016 21:13                   -
                        <a href="2.7.14/">2.7.14/</a>                                            16-Sep-2017 20:35                   -
                        <a href="2.7.15/">2.7.15/</a>                                            30-Apr-2018 16:47                   -
                        <a href="2.7.2/">2.7.2/</a>                                             20-Mar-2014 21:57                   -
                        <a href="2.7.3/">2.7.3/</a>                                             20-Mar-2014 21:57                   -
                        <a href="2.7.4/">2.7.4/</a>                                             23-Jul-2013 12:17                   -
                        <a href="2.7.5/">2.7.5/</a>                                             23-Jul-2013 12:18                   -
                        <a href="2.7.6/">2.7.6/</a>                                             10-Nov-2013 18:45                   -
                        <a href="2.7.7/">2.7.7/</a>                                             02-Jul-2014 04:32                   -
                        <a href="2.7.8/">2.7.8/</a>                                             02-Jul-2014 04:42                   -
                        <a href="2.7.9/">2.7.9/</a>                                             10-Dec-2014 22:12                   -
                        <a href="3.0/">3.0/</a>                                               20-Mar-2014 22:00                   -
                        <a href="3.0.1/">3.0.1/</a>                                             20-Mar-2014 22:00                   -
                        <a href="3.1/">3.1/</a>                                               20-Mar-2014 21:57                   -
                        <a href="3.1.1/">3.1.1/</a>                                             20-Mar-2014 21:58                   -
                        <a href="3.1.2/">3.1.2/</a>                                             20-Mar-2014 21:57                   -
                        <a href="3.1.3/">3.1.3/</a>                                             20-Mar-2014 21:58                   -
                        <a href="3.1.4/">3.1.4/</a>                                             20-Mar-2014 21:57                   -
                        <a href="3.1.5/">3.1.5/</a>                                             20-Mar-2014 21:57                   -
                        <a href="3.2/">3.2/</a>                                               04-Mar-2012 15:49                   -
                        <a href="3.2.1/">3.2.1/</a>                                             04-Mar-2012 15:48                   -
                        <a href="3.2.2/">3.2.2/</a>                                             04-Mar-2012 15:48                   -
                        <a href="3.2.3/">3.2.3/</a>                                             28-Feb-2014 07:04                   -
                        <a href="3.2.4/">3.2.4/</a>                                             28-Feb-2014 07:08                   -
                        <a href="3.2.5/">3.2.5/</a>                                             15-May-2013 22:14                   -
                        <a href="3.2.6/">3.2.6/</a>                                             12-Oct-2014 07:12                   -
                        <a href="3.3.0/">3.3.0/</a>                                             12-Nov-2013 07:03                   -
                        <a href="3.3.1/">3.3.1/</a>                                             28-Feb-2014 07:08                   -
                        <a href="3.3.2/">3.3.2/</a>                                             23-Jul-2013 12:18                   -
                        <a href="3.3.3/">3.3.3/</a>                                             19-Nov-2013 06:57                   -
                        <a href="3.3.4/">3.3.4/</a>                                             10-Feb-2014 17:45                   -
                        <a href="3.3.5/">3.3.5/</a>                                             09-Mar-2014 09:58                   -
                        <a href="3.3.6/">3.3.6/</a>                                             12-Oct-2014 07:23                   -
                        <a href="3.3.7/">3.3.7/</a>                                             19-Sep-2017 08:06                   -
                        <a href="3.4.0/">3.4.0/</a>                                             07-Aug-2017 07:08                   -
                        <a href="3.4.1/">3.4.1/</a>                                             19-May-2014 05:27                   -
                        <a href="3.4.2/">3.4.2/</a>                                             08-Oct-2014 08:49                   -
                        <a href="3.4.3/">3.4.3/</a>                                             25-Feb-2015 11:40                   -
                        <a href="3.4.4/">3.4.4/</a>                                             21-Dec-2015 06:15                   -
                        <a href="3.4.5/">3.4.5/</a>                                             25-Jun-2016 22:06                   -
                        <a href="3.4.6/">3.4.6/</a>                                             17-Jan-2017 08:14                   -
                        <a href="3.4.7/">3.4.7/</a>                                             09-Aug-2017 07:25                   -
                        <a href="3.4.8/">3.4.8/</a>                                             05-Feb-2018 00:17                   -
                        <a href="3.4.9/">3.4.9/</a>                                             02-Aug-2018 13:17                   -
                        <a href="3.5.0/">3.5.0/</a>                                             13-Sep-2015 11:56                   -
                        <a href="3.5.1/">3.5.1/</a>                                             07-Dec-2015 01:58                   -
                        <a href="3.5.2/">3.5.2/</a>                                             27-Jun-2016 18:41                   -
                        <a href="3.5.3/">3.5.3/</a>                                             17-Jan-2017 08:14                   -
                        <a href="3.5.4/">3.5.4/</a>                                             08-Aug-2017 10:41                   -
                        <a href="3.5.5/">3.5.5/</a>                                             05-Feb-2018 00:18                   -
                        <a href="3.5.6/">3.5.6/</a>                                             02-Aug-2018 13:23                   -
                        <a href="3.6.0/">3.6.0/</a>                                             23-Dec-2016 09:25                   -
                        <a href="3.6.1/">3.6.1/</a>                                             21-Mar-2017 21:57                   -
                        <a href="3.6.2/">3.6.2/</a>                                             17-Jul-2017 04:10                   -
                        <a href="3.6.3/">3.6.3/</a>                                             03-Oct-2017 18:36                   -
                        <a href="3.6.4/">3.6.4/</a>                                             19-Dec-2017 07:20                   -
                        <a href="3.6.5/">3.6.5/</a>                                             28-Mar-2018 17:35                   -
                        <a href="3.6.6/">3.6.6/</a>                                             27-Jun-2018 06:00                   -
                        <a href="3.6.7/">3.6.7/</a>                                             20-Oct-2018 15:25                   -
                        <a href="3.6.8/">3.6.8/</a>                                             24-Dec-2018 08:27                   -
                        <a href="3.7.0/">3.7.0/</a>                                             27-Jun-2018 06:12                   -
                        <a href="3.7.1/">3.7.1/</a>                                             20-Oct-2018 15:28                   -
                        <a href="3.7.2/">3.7.2/</a>                                             24-Dec-2018 08:29                   -
                        <a href="binaries-1.1/">binaries-1.1/</a>                                      06-Aug-2001 02:17                   -
                        <a href="binaries-1.2/">binaries-1.2/</a>                                      06-Aug-2001 02:14                   -
                        <a href="binaries-1.3/">binaries-1.3/</a>                                      06-Aug-2001 02:17                   -
                        <a href="binaries-1.4/">binaries-1.4/</a>                                      06-Aug-2001 02:17                   -
                        <a href="binaries-1.5/">binaries-1.5/</a>                                      06-Aug-2001 02:17                   -
                        <a href="contrib/">contrib/</a>                                           29-Apr-2005 12:31                   -
                        <a href="contrib-09-Dec-1999/">contrib-09-Dec-1999/</a>                               26-Dec-2001 05:53                   -
                        <a href="devtest/">devtest/</a>                                           21-Dec-2018 22:50                   -
                        <a href="doc/">doc/</a>                                               24-Dec-2018 08:52                   -
                        <a href="mac/">mac/</a>                                               11-Oct-2005 07:18                   -
                        <a href="mail/">mail/</a>                                              02-Aug-2001 21:48                   -
                        <a href="misc/">misc/</a>                                              11-Jul-2004 02:03                   -
                        <a href="nt/">nt/</a>                                                11-Oct-2005 07:18                   -
                        <a href="parrotbench/">parrotbench/</a>                                       31-Dec-2003 17:49                   -
                        <a href="pc/">pc/</a>                                                02-Aug-2001 21:48                   -
                        <a href="pythonwin/">pythonwin/</a>                                         04-Sep-2003 23:32                   -
                        <a href="src/">src/</a>                                               12-Jan-2015 04:20                   -
                        <a href="vms/">vms/</a>                                               06-Aug-2001 02:17                   -
                        <a href="win32/">win32/</a>                                             23-Feb-2011 01:27                   -
                        <a href="wpy/">wpy/</a>                                               04-Sep-2003 23:33                   -
                        <a href="INDEX">INDEX</a>                                              06-Aug-2001 12:34                1789
                        <a href="README">README</a>                                             06-Aug-2001 12:33                1789
                        <a href="README.html">README.html</a>                                        06-Aug-2001 12:34                2378
                        </pre><hr></body>
                        </html>"#;

        let parsed = parse_index_html(index_html).unwrap();

        let expected: Vec<Version> = vec![
            "2.0.0".parse().unwrap(),
            "2.0.1".parse().unwrap(),
            "2.1.0".parse().unwrap(),
            "2.1.1".parse().unwrap(),
            "2.1.2".parse().unwrap(),
            "2.1.3".parse().unwrap(),
            "2.2.0".parse().unwrap(),
            "2.2.1".parse().unwrap(),
            "2.2.2".parse().unwrap(),
            "2.2.3".parse().unwrap(),
            "2.3.0".parse().unwrap(),
            "2.3.1".parse().unwrap(),
            "2.3.2".parse().unwrap(),
            "2.3.3".parse().unwrap(),
            "2.3.4".parse().unwrap(),
            "2.3.5".parse().unwrap(),
            "2.3.6".parse().unwrap(),
            "2.3.7".parse().unwrap(),
            "2.4.0".parse().unwrap(),
            "2.4.1".parse().unwrap(),
            "2.4.2".parse().unwrap(),
            "2.4.3".parse().unwrap(),
            "2.4.4".parse().unwrap(),
            "2.4.5".parse().unwrap(),
            "2.4.6".parse().unwrap(),
            "2.5.0".parse().unwrap(),
            "2.5.1".parse().unwrap(),
            "2.5.2".parse().unwrap(),
            "2.5.3".parse().unwrap(),
            "2.5.4".parse().unwrap(),
            "2.5.5".parse().unwrap(),
            "2.5.6".parse().unwrap(),
            "2.6.0".parse().unwrap(),
            "2.6.1".parse().unwrap(),
            "2.6.2".parse().unwrap(),
            "2.6.3".parse().unwrap(),
            "2.6.4".parse().unwrap(),
            "2.6.5".parse().unwrap(),
            "2.6.6".parse().unwrap(),
            "2.6.7".parse().unwrap(),
            "2.6.8".parse().unwrap(),
            "2.6.9".parse().unwrap(),
            "2.7.0".parse().unwrap(),
            "2.7.1".parse().unwrap(),
            "2.7.10".parse().unwrap(),
            "2.7.11".parse().unwrap(),
            "2.7.12".parse().unwrap(),
            "2.7.13".parse().unwrap(),
            "2.7.14".parse().unwrap(),
            "2.7.15".parse().unwrap(),
            "2.7.2".parse().unwrap(),
            "2.7.3".parse().unwrap(),
            "2.7.4".parse().unwrap(),
            "2.7.5".parse().unwrap(),
            "2.7.6".parse().unwrap(),
            "2.7.7".parse().unwrap(),
            "2.7.8".parse().unwrap(),
            "2.7.9".parse().unwrap(),
            "3.0.0".parse().unwrap(),
            "3.0.1".parse().unwrap(),
            "3.1.0".parse().unwrap(),
            "3.1.1".parse().unwrap(),
            "3.1.2".parse().unwrap(),
            "3.1.3".parse().unwrap(),
            "3.1.4".parse().unwrap(),
            "3.1.5".parse().unwrap(),
            "3.2.0".parse().unwrap(),
            "3.2.1".parse().unwrap(),
            "3.2.2".parse().unwrap(),
            "3.2.3".parse().unwrap(),
            "3.2.4".parse().unwrap(),
            "3.2.5".parse().unwrap(),
            "3.2.6".parse().unwrap(),
            "3.3.0".parse().unwrap(),
            "3.3.1".parse().unwrap(),
            "3.3.2".parse().unwrap(),
            "3.3.3".parse().unwrap(),
            "3.3.4".parse().unwrap(),
            "3.3.5".parse().unwrap(),
            "3.3.6".parse().unwrap(),
            "3.3.7".parse().unwrap(),
            "3.4.0".parse().unwrap(),
            "3.4.1".parse().unwrap(),
            "3.4.2".parse().unwrap(),
            "3.4.3".parse().unwrap(),
            "3.4.4".parse().unwrap(),
            "3.4.5".parse().unwrap(),
            "3.4.6".parse().unwrap(),
            "3.4.7".parse().unwrap(),
            "3.4.8".parse().unwrap(),
            "3.4.9".parse().unwrap(),
            "3.5.0".parse().unwrap(),
            "3.5.1".parse().unwrap(),
            "3.5.2".parse().unwrap(),
            "3.5.3".parse().unwrap(),
            "3.5.4".parse().unwrap(),
            "3.5.5".parse().unwrap(),
            "3.5.6".parse().unwrap(),
            "3.6.0".parse().unwrap(),
            "3.6.1".parse().unwrap(),
            "3.6.2".parse().unwrap(),
            "3.6.3".parse().unwrap(),
            "3.6.4".parse().unwrap(),
            "3.6.5".parse().unwrap(),
            "3.6.6".parse().unwrap(),
            "3.6.7".parse().unwrap(),
            "3.6.8".parse().unwrap(),
            "3.7.0".parse().unwrap(),
            "3.7.1".parse().unwrap(),
            "3.7.2".parse().unwrap(),
        ];
        assert_eq!(parsed, expected);
    }
}
