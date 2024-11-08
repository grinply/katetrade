use std::{env};
use std::fs::File;
use std::io::{Read, Write};
use std::time::Duration;
use rusty_s3::{Bucket, Credentials, S3Action, UrlStyle};
use ureq::Error;

pub struct AwsClient {
    credentials: Credentials,
    region: String,
}

const AWS_S3_ENDPOINT: &str = "https://s3.amazonaws.com";

impl AwsClient {
    pub fn new(region: String) -> Result<AwsClient, anyhow::Error> {
        let key = env::var("AWS_ACCESS_KEY_ID");
        let secret =  env::var("AWS_SECRET_ACCESS_KEY");

        if key.is_err() || secret.is_err() {
            anyhow::bail!("missing aws credentials on environment variables 'AWS_ACCESS_KEY_ID' and 'AWS_SECRET_ACCESS_KEY'");
        }

        Ok(AwsClient {
            credentials: Credentials::new(key?, secret?),
            region,
        })
    }

    pub fn file_exists_on_s3(&self, bucket_name: &str, s3_file_path: &str) -> Result<bool, anyhow::Error> {
        let region = self.region.clone();
        let bucket = Bucket::new(
            AWS_S3_ENDPOINT.parse()?,
            UrlStyle::VirtualHost,
            bucket_name.to_string(),
            region)?;

        let timeout_seconds: u64 = 30 * 60;
        let head_object = bucket.head_object(Some(&self.credentials), s3_file_path);
        let signed_url = head_object.sign(Duration::from_secs(timeout_seconds));

        let url = signed_url.as_str();

        let response_result = ureq::head(url)
            .timeout(Duration::from_secs(timeout_seconds))
            .call();

        match response_result {
            Ok(response) => Ok(response.status() == 200),
            Err(Error::Status(code, _)) => {
                if code == 404 {
                    Ok(false)
                }else{
                    anyhow::bail!("failed to correctly verify if file exists on aws s3, response code {}", code)
                }
            },
            _ => unreachable!()
        }
    }

    pub fn download_file_from_s3(&self,
                                 bucket_name: &str,
                                 s3_file_path: &str,
                                 file_output_location: &str) -> Result<(), anyhow::Error> {
        let region = self.region.clone();
        let bucket = Bucket::new(
            AWS_S3_ENDPOINT.parse()?,
            UrlStyle::VirtualHost,
            bucket_name.to_string(),
            region)?;

        let s3_file_path = if s3_file_path.starts_with("/") {
            s3_file_path.to_string()
        }else{
            format!("/{}", s3_file_path)
        };

        let timeout_seconds: u64 = 10 * 60;
        let get_object = bucket.get_object(Some(&self.credentials), s3_file_path.as_str());
        let signed_url = get_object.sign(Duration::from_secs(timeout_seconds));

        let response = ureq::get(signed_url.as_str())
            .timeout(Duration::from_secs(timeout_seconds))
            .call()?;

        assert!(response.has("Content-Length"));
        let len: usize = response.header("Content-Length")
            .unwrap()
            .parse()?;

        let mut bytes: Vec<u8> = Vec::with_capacity(len);
        response.into_reader()
            .take((len + 1) as u64)
            .read_to_end(&mut bytes)?;

        let mut file = File::create(file_output_location)?;
        file.write_all(bytes.as_slice())?;
        file.sync_all()?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[ignore = "will only run in the CI/CD where proper credentials are available for integration testing"]
    fn should_download_file_from_s3_correctly() {
        let temp_dir = env::temp_dir();
        let bucket_name = "profitpirates".to_string();
        let file = "/trades/IND/TRADE-B3-FUTURES-IND-20240614";
        let output_dir = format!("{}/{}",
                                 temp_dir.as_path().to_str().unwrap(),
                                 "TRADE-B3-FUTURES-IND-20240614"
        );

        println!("File saved to : {}", output_dir);
        let aws = AwsClient::new("sa-east-1".to_string()).unwrap();
        aws.download_file_from_s3(bucket_name.as_str(), file, output_dir.as_str()).unwrap();
    }

    #[test]
    #[ignore = "will only run in the CI/CD where proper credentials are available for integration testing"]
    fn should_check_that_file_exists_on_s3_correctly() {
        let bucket_name = "profitpirates".to_string();
        let file = "trades/IND/TRADE-B3-FUTURES-IND-20240614";

        let aws = AwsClient::new("sa-east-1".to_string()).unwrap();
        let exists_on_s3 = aws.file_exists_on_s3(bucket_name.as_str(), file);

        assert!(exists_on_s3.is_ok(), "should not produce an error");
        assert!(exists_on_s3.unwrap(), "file should exists on profitpirates s3 bucket");
    }

    #[test]
    #[ignore = "will only run in the CI/CD where proper credentials are available for integration testing"]
    fn should_check_that_file_doesnt_exists_on_s3_correctly() {
        let bucket_name = "profitpirates".to_string();
        let file = "trades/FILE_NOT_FOUND";

        let aws = AwsClient::new("sa-east-1".to_string()).unwrap();
        let exists_on_s3 = aws.file_exists_on_s3(bucket_name.as_str(), file);

        assert!(exists_on_s3.is_ok(), "should not produce an error");
        assert!(!exists_on_s3.unwrap(), "file should NOT exist on profitpirates s3 bucket");
    }
}
