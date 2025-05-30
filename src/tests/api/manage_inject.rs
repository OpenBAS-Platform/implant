#[cfg(test)]
mod tests {
    use mockito;
    use std::fs;
    use std::io::Read;

    #[test]
    fn test_download_file_in_memory_success() {
        // -- PREPARE --
        let mut server = mockito::Server::new();
        let server_url = server.url();

        let filename = "test.txt";
        let file_content = "Hello, OpenBAS!";
        let content_disposition = format!("attachment; filename=\"{}\"", filename);

        let _m = server
            .mock("GET", "/api/documents/123/file")
            .with_status(200)
            .with_header("content-disposition", &content_disposition)
            .with_body(file_content)
            .create();

        let client = crate::api::Client::new(
            server_url,
            crate::tests::api::client::TOKEN_TEST.to_string(),
            false,
            false,
        );

        // -- EXECUTE --
        let result = client.download_file(&"123".to_string(), true);

        // -- ASSERT --
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), filename);
    }

    #[test]
    fn test_download_file_to_disk_success() {
        // -- PREPARE --
        let mut server = mockito::Server::new();
        let server_url = server.url();

        let filename = "test.txt";
        let file_content = "Hello, OpenBAS!";
        let content_disposition = format!("attachment; filename=\"{}\"", filename);

        let _m = server
            .mock("GET", "/api/documents/123/file")
            .with_status(200)
            .with_header("content-disposition", &content_disposition)
            .with_body(file_content)
            .create();

        let client = crate::api::Client::new(
            server_url,
            crate::tests::api::client::TOKEN_TEST.to_string(),
            false,
            false,
        );

        // -- EXECUTE --
        let result = client.download_file(&"123".to_string(), false);

        // -- ASSERT --
        assert!(result.is_ok());

        let current_exe_path = std::env::current_exe().unwrap();
        let expected_file_path = current_exe_path.parent().unwrap().join(filename);

        assert!(expected_file_path.exists());

        let mut content = String::new();
        let mut file = fs::File::open(&expected_file_path).unwrap();
        file.read_to_string(&mut content).unwrap();
        assert_eq!(content, file_content);

        // -- CLEAN --
        fs::remove_file(expected_file_path).unwrap();
    }
}
