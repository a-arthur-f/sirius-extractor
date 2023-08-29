use async_std::{io::WriteExt, task};
use futures::future;
use json::{self, JsonValue};
use reqwest::{Client, Url};
use std::{env, fs, path::Path, time::Duration};
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Por favor insira a URL do livro");
        return;
    }

    let url = Url::parse(&args[1]);

    match url {
        Err(_) => {
            println!("URL invalida, tente novamente");
            return;
        }

        Ok(url) => {
            let query = url.query().unwrap_or("");
            task::block_on(async {
                let arquivo_url = format!(
                    "http://{}{}?{}",
                    url.domain().unwrap(),
                    url.path().replace("prima-pdf", "prima-arquivo-pdf"),
                    query
                );

                match get_request_json(&arquivo_url).await {
                    Ok(res_json) => {
                        let paginas = &res_json["Paginas"];
                        println!("{paginas}");

                        extract_pages(&url, paginas.as_str().unwrap().parse().unwrap())
                            .await
                            .unwrap();
                    }

                    Err(_) => {
                        println!("Falha ao extrair dados");
                        return;
                    }
                }
            })
        }
    }
}

async fn get_request_json(url: &str) -> Result<JsonValue, Box<dyn std::error::Error>> {
    let res = reqwest::get(url).await?.text().await?;
    let res_json = json::parse(&res)?;

    Ok(res_json)
}

async fn extract_pages(url: &Url, paginas: u32) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let tstmp = "1693233550909";
    let pagina_url = format!(
        "http://{}{}?{}&tstmp={tstmp}",
        url.domain().unwrap(),
        url.path().replace("prima-pdf", "prima-pagina-pdf"),
        url.query().unwrap()
    );

    match Path::new("images").exists() {
        true => {
            fs::remove_dir_all("images")?;
            fs::create_dir("images")?
        }
        false => {
            fs::create_dir("images")?;
        }
    }

    let mut futures = vec![];

    for i in 1..=paginas {
        futures.push(download(&client, &pagina_url, i));
    }

    future::join_all(futures).await;

    Ok(())
}

async fn download(
    client: &Client,
    pagina_url: &str,
    i: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = vec![];
    loop {
        let pagina_url = String::from(format!("{pagina_url}&pagina={i}"));
        let res = client.get(&pagina_url).send().await?;
        let res = res.bytes().await?;
        if res.len() == 0 {
            task::sleep(Duration::from_millis(500)).await;
            continue;
        } else {
            bytes = res.to_vec();
            break;
        }
    }

    let mut file = async_std::fs::File::create(format!("images/image{i}.png")).await?;
    println!("Baixando a página {i}");
    file.write_all(&bytes).await?;
    Ok(())
}
