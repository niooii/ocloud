use futures::future::join_all;
use nanoid::nanoid;
use reqwest::{multipart::{Form, Part}, StatusCode};
use tokio::join;

pub async fn race_condition() {

    let unique = nanoid!(6);

    let form1 = Form::new()
        .file("somefile", "Cargo.toml").await
        .unwrap();

    let form2 = Form::new()
        .file("somefile", "Cargo.toml").await
        .unwrap();

    let client = reqwest::Client::new();
    let req1 = client.post(format!("http://127.0.0.1:9100/media/test_{unique}/"))
        .multipart(form1).send();
    let req2 = client.post(format!("http://127.0.0.1:9100/media/test_{unique}/"))
        .multipart(form2).send();

    let res = join_all(vec![req1, req2])
        .await;
    let res = res.iter().map(|f| f.as_ref().unwrap().status());
    println!("{:?}", res.collect::<Vec<StatusCode>>());

    let get_res = client.get(format!("http://127.0.0.1:9100/media/test_{unique}/somefile"))
        .send().await.unwrap();

    println!("got media back: {}", get_res.status());
    
    let del_res = client.delete(format!("http://127.0.0.1:9100/media/test_{unique}/somefile"))
        .send().await.unwrap();

    println!("deleted media: {}", del_res.status());
}
