
use tonic::{transport::Server, Request, Response};
use tokio::sync::mpsc;
use std::path::Path;
use std::fs::File;
use std::io::BufWriter;
use std::io::BufReader;
use std::io::Write;
use std::io::Read;
use tokio::sync::mpsc::Receiver;
use futures_core::stream::Stream;
use std::pin::Pin;
use std::task::Poll;
use std::task::Context;

const IMAGE_SERVER_ADDRESS: &str = "192.168.0.100:50053";
const USER_IMAGES_DIR: &str = "user_imgs";

mod imagesservice;
use imagesservice::{UploadImageRequest, UploadImageResponse, DownloadImageRequest,
    DownloadImageResponse, RemoveImageRequest, RemoveImageResponse};
use imagesservice::images_server::{Images, ImagesServer};

struct StreamReceiver<T>{
    inner_rx: Receiver<T>
}
impl<T> StreamReceiver<T> {
    pub fn new(recv: Receiver<T>) -> Self {
        Self { inner_rx: recv }
    }
}
impl<T> Stream for StreamReceiver<T> {
    type Item = T;
    
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        //return Pin::new(&mut self.inner_rx).poll_recv(cx);
        return self.inner_rx.poll_recv(cx);
    }
}

#[derive(Default)]
struct ImageService {

}

#[tonic::async_trait]
impl Images for ImageService {
    async fn upload_image(
        &self,
        request: Request<tonic::Streaming<UploadImageRequest>>,
    ) -> Result<Response<UploadImageResponse>, tonic::Status>
    {
        println!("upload_image request");
        let mut stream = request.into_inner();
        use std::fs;
        let user_imgs_path = Path::new(USER_IMAGES_DIR);
        let create_dir_res = fs::create_dir(&user_imgs_path);
        match create_dir_res {
            Err(err) => println!("{:?}", err.kind()),
            Ok(_) => {}
        }

        use tokio_stream::StreamExt;
        if let Some(upload_image_request_result) = stream.next().await {
            if let Ok(upload_image_request) = upload_image_request_result {
                let user_id_from_request = upload_image_request.user_id;
                println!("upload_image: user_id_from_request={}",&user_id_from_request);
                let file_name_from_request = upload_image_request.image_name;
                write_image_file_name_to_db(&user_id_from_request, &file_name_from_request);
                println!("upload_image: file_name_from_request={}",&file_name_from_request);
                // check if file with same file_name not exists
                let file_name_path = Path::new(&file_name_from_request);
                let file_name_in_user_imgs_path = user_imgs_path.join(file_name_path);
                if file_name_in_user_imgs_path.exists() == false {
                    let file = File::create(&file_name_in_user_imgs_path)?;
                    let mut buf_writer = BufWriter::new(file);
                    /*let connected_clients = &mut (*(self.connected_clients.write().await));
                    if let Some(connected_client) = connected_clients.get_mut(&user_id_from_request) {
                        connected_client.image_name = Some(file_name_from_request);
                    }*/
                    let file_chunk = upload_image_request.file_chunk;
                    let res = buf_writer.write_all(file_chunk.as_slice());

                    if let Err(err) = res {
                        println!("{:?}", err.kind());
                    }
            
                    while let Some(upload_image_request_result) = stream.next().await {
                        if let Ok(upload_image_request) = upload_image_request_result {
                            let file_chunk = upload_image_request.file_chunk;
                            let res = buf_writer.write_all(file_chunk.as_slice());
                            if let Err(err) = res {
                                println!("{:?}", err.kind());
                            }
                        }
                    }
                    let res = buf_writer.flush();
                    if let Err(err) = res {
                        println!("error: {}", err);
                    }
                    println!("upload_image: upload success");
                } else {
                    println!("upload_image: file with same name exists");
                    //todo: create another file name
                }
            }
        }

        let reply = UploadImageResponse{
        };
        return Ok(Response::new(reply));
    }
    
    type DownloadImageStream = StreamReceiver<Result<DownloadImageResponse, tonic::Status>>;
    async fn download_image(
        &self,
        request: tonic::Request<DownloadImageRequest>,
    ) -> Result<tonic::Response<Self::DownloadImageStream>, tonic::Status> {
        let (tx, rx) = mpsc::channel(10000);

        let user_id_from_request = request.get_ref().user_id.clone();

        //let connected_clients = &(*(self.connected_clients.read().await));
        //if let Some(connected_client) = connected_clients.get(&user_id_from_request) {
            let image_name: String;
            //if let Some(image_name_ref) = &connected_client.image_name {
            //    image_name = image_name_ref.clone();
            //} else {
                image_name = read_image_file_name_from_db(&user_id_from_request);
            //}
            if image_name != "" {
                let user_imgs_path = Path::new(USER_IMAGES_DIR);
                let file_name_path = Path::new(&image_name);
                let file_name_in_user_imgs_path = user_imgs_path.join(file_name_path);
                if let Ok(file) = File::open(&file_name_in_user_imgs_path) {
                    let mut buf_reader = BufReader::new(file);
                    let buffer_size = 1024;
                    let mut buf: Vec<u8> = vec![0; buffer_size];

                    if let Ok(mut res) = buf_reader.read(buf.as_mut_slice()) {
                        while res > 0 {
                            let reply = DownloadImageResponse {
                                response_code: 1,
                                file_chunk: buf.clone()
                            };
                            let tx_tmp = tx.clone();
                            /*let join_handle = tokio::task::spawn(async move{
                                let result = tx_tmp.send(Ok(reply)).await;
                                match result {
                                    Ok(_) =>{println!("{}", counter);},
                                    Err(e) =>{
                                        println!(" download_image ERROR: {}", e);
                                    }
                                }
                            });
                            join_handle.await;*/
                            let result = tx_tmp.send(Ok(reply)).await;
                            match result {
                                Ok(_) =>{},
                                Err(e) =>{
                                    println!(" download_image ERROR: {}", e);
                                }
                            }
                            /*tokio::spawn(async move {
                                let result = tx_tmp.send(Ok(reply)).await;
                                match result {
                                    Ok(_) =>{println!("{}", counter);},
                                    Err(e) =>{
                                        println!(" download_image ERROR: {}", e);
                                    }
                                }
                            }).await.expect("Error while sending message");*/
                            if let Ok(n) = buf_reader.read(buf.as_mut_slice()) {
                                res = n;
                            } else {
                                println!(" download_image readfile ERROR: ");
                                let reply = DownloadImageResponse {
                                    response_code: -1,
                                    file_chunk: vec![]
                                };
                                let tx_tmp = tx.clone();
                                let join_handle = tokio::spawn(async move {
                                    let result = tx_tmp.send(Ok(reply)).await;
                                    match result {
                                        Ok(_) =>{},
                                        Err(e) =>{
                                            println!(" download_image ERROR: {}", e)
                                        }
                                    }
                                });
                                let res = join_handle.await;
                                if let Err(err) = res {
                                    println!("error: {}", err);
                                }
                            }
                        }
                        if res == 0 {
                            println!("finished");
                            let reply = DownloadImageResponse {
                                response_code: 2,
                                file_chunk: buf.clone()
                            };
                            let tx_tmp = tx.clone();
                            let result = tx_tmp.send(Ok(reply)).await;
                            match result {
                                Ok(_) =>println!("download_image: finished download_image"),
                                Err(e) =>println!(" download_image ERROR: {}", e)
                            }
                            /*tokio::spawn(async move {
                                let result = tx_tmp.send(Ok(reply)).await;
                                match result {
                                    Ok(_) =>println!("download_image: finished download_image"),
                                    Err(e) =>println!(" download_image ERROR: {}", e)
                                }
                            });*/
                        }
                    } else {
                        println!("Error");
                        let reply = DownloadImageResponse {
                            response_code: -1,
                            file_chunk: vec![]
                        };
                        let tx_tmp = tx.clone();
                        tokio::spawn(async move {
                            let res = tx_tmp.send(Ok(reply)).await;
                            match res {
                                Ok(_) =>println!("download_image: sent a download_image"),
                                Err(e) =>println!(" download_image ERROR: {}", e)
                            }
                        });
                    }
                } else {
                    println!("download_image: error while opening file");
                }
            } else {
                println!("download_image: no image_name");
            }
            
        //} else {
          //  println!("download_image: no connected_client");
        //}

        let stream_receiver = StreamReceiver::new(rx);
        return Ok(Response::new(stream_receiver));
    }
    async fn remove_image(
        &self,
        request: Request<RemoveImageRequest>,
    ) -> Result<tonic::Response<RemoveImageResponse>, tonic::Status> {
        //todo: remove image_name from connected client user_id
        //todo: remove image from USER_IMAGES_DIR
        use std::fs;
        let user_id_from_request = request.get_ref().user_id.clone();
        //let connected_clients = &mut (*(self.connected_clients.write().await));
        let mut image_name = "".to_string();
        /*if let Some(connected_client) = connected_clients.get_mut(&user_id_from_request) {
            if let Some(image_name_ref) = &mut connected_client.image_name {
                image_name = image_name_ref.clone();
            
                connected_client.image_name = Option::None;
            }
        }*/
        if image_name == "" {
            image_name = read_image_file_name_from_db(&user_id_from_request);
        }
        if image_name != "" {
            let user_imgs_path = Path::new(USER_IMAGES_DIR);
            let file_name_path = Path::new(&image_name);
            let file_name_in_user_imgs_path = user_imgs_path.join(file_name_path);
            if file_name_in_user_imgs_path.exists() == true {
                let res = fs::remove_file(&file_name_in_user_imgs_path);
                if let Err(_) = res {
                    println!("error while removing file");
                }
            }
        }

        write_image_file_name_to_db(&user_id_from_request, &"".to_string());

        let reply = RemoveImageResponse {
            response_code: 1
        };
        return Ok(Response::new(reply));
    }
}

fn write_image_file_name_to_db(user_id: &String, file_name: &String) {
    use pickledb::{PickleDb, PickleDbDumpPolicy, SerializationMethod};
    let user_imgs_path = Path::new(USER_IMAGES_DIR);
    let db_file_name_path = Path::new("users.db");
    let file_name_in_user_imgs_path = user_imgs_path.join(db_file_name_path);
    if Path::new(&user_imgs_path).exists() {
        if Path::new(&file_name_in_user_imgs_path).exists() {
            let db_res = PickleDb::load(&file_name_in_user_imgs_path, PickleDbDumpPolicy::DumpUponRequest, SerializationMethod::Json);
            if let Ok(mut db) = db_res {
                let res = db.set(user_id, file_name);
                if let Ok(_) = res {
                    println!("The image of {} loaded from file", user_id);
                }
            } else {
                let mut db = PickleDb::new(&file_name_in_user_imgs_path, PickleDbDumpPolicy::AutoDump, SerializationMethod::Json);
                let res = db.set(user_id, file_name);
                if let Ok(_) = res {
                    println!("The image of {} loaded from file", user_id);
                    //println!("The image of {} is: {}", user_id, db.get::<String>(user_id));
                }
            }
        } else {
            let mut db = PickleDb::new(&file_name_in_user_imgs_path, PickleDbDumpPolicy::AutoDump, SerializationMethod::Json);
            let res = db.set(user_id, file_name);
            if let Ok(_) = res {
                println!("The image of {} loaded from file", user_id);
            }
        }
    }
}

fn read_image_file_name_from_db(user_id: &String)->String {
    use pickledb::{PickleDb, PickleDbDumpPolicy, SerializationMethod};
    let user_imgs_path = Path::new(USER_IMAGES_DIR);
    let db_file_name_path = Path::new("users.db");
    let file_name_in_user_imgs_path = user_imgs_path.join(db_file_name_path);

    let mut found_file_name = String::from("");
    if Path::new(&file_name_in_user_imgs_path).exists() {
        let db_res = PickleDb::load(&file_name_in_user_imgs_path, PickleDbDumpPolicy::DumpUponRequest, SerializationMethod::Json);
        if let Ok(db) = db_res {
            if let Some(file_name) = db.get::<String>(user_id) {
                found_file_name = file_name;
            }
        }
    }
    
    return found_file_name;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = IMAGE_SERVER_ADDRESS.parse()?;
    let image_service = ImageService::default();

    println!("ImageServer listening on {}", addr);

    Server::builder()
        .add_service(ImagesServer::new(image_service))
        .serve(addr)
        .await?;

    return Ok(());
}
