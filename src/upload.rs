pub mod Upload {

    use actix_multipart::Multipart;
    use actix_web::{
        web::{self, Buf},
        Error, HttpResponse,
    };
    use futures_util::TryStreamExt as _;
    use std::io::Write;
    use uuid::Uuid;

    #[derive(Debug, Clone)]
    pub struct FileResponse {
        name: String,
        value: Vec<u8>,
        file_name: String,
        ext: String,
    }

    #[derive(Debug, Clone)]
    pub struct TextResponse {
        name: String,
        value: String,
    }

    #[derive(Debug,Clone)]
    pub enum MultipartResponse {
        FILE(FileResponse),
        TEXT(TextResponse),
    }

    pub async fn save_file(
        mut payload: Multipart,
        destination_dir: &str,
    ) -> Result< Vec<MultipartResponse>, Box<dyn std::error::Error>>{
        let mut response: Vec<MultipartResponse> = Vec::new();

        while let Some(mut field) = payload.try_next().await? {
            let fieldname = field.name().clone().to_string();

            let mut value_buff: Vec<u8> = Vec::new();

            while let Some(mut chunk) = field.try_next().await? {
                let mut ch: Vec<u8> = chunk.to_vec();
                value_buff.append(&mut ch);
            }

            let content_disposition = field.content_disposition();
            let content_type = field.content_type();

            if let None = content_type {

                let mut value = std::str::from_utf8(&value_buff).unwrap_or("").to_owned();
                let mut res = MultipartResponse::TEXT(TextResponse {
                    name: fieldname.clone(),
                    value: value,
                });

                response.push(res);

                continue;
            }

            let filename = content_disposition
                .get_filename()
                .map_or_else(|| Uuid::new_v4().to_string(), sanitize_filename::sanitize);

            let ext = content_disposition
                .get_filename_ext()
                .map_or("".to_string(), |e| e.to_string());

            let filepath = format!("{destination_dir}/{filename}");

            // File::create is blocking operation, use threadpool
            let mut f = web::block(|| std::fs::File::create(filepath)).await??;

            let res = MultipartResponse::FILE(FileResponse {
                name: fieldname.clone(),
                value: value_buff,
                file_name: filename,
                ext: ext,
            });
            response.push(res.clone());

            if let MultipartResponse::FILE(r) = res {
                let f = web::block(move || f.write_all(&r.value.clone()).map(|_| f)).await??;
            }
        }

        return Ok(response);
    }
}
