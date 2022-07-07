use std::{
    cmp::{max, min},
    collections::HashMap,
    io::Result,
    net::SocketAddr,
    str::FromStr,
    sync::Arc,
};

use axum::{
    extract::{Extension, Path},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Record {
    rank: usize,
    word: String,
    pro: String,
    pos: String,
    tag: String,
    definition: String,
}

fn load_dictionary(path: &str) -> Result<HashMap<String, Record>> {
    let mut map = HashMap::new();
    let mut reader = csv::Reader::from_path(path)?;
    for result in reader.deserialize() {
        let r: Record = result?;
        if !map.contains_key(&r.word) {
            map.insert(r.word.clone(), r);
        }
    }
    Ok(map)
}

fn suggest_words(word: &str, limit: usize, records: &HashMap<String, Record>) -> Vec<String> {
    let mut words = Vec::new();

    for (k, _) in records {
        if words.len() >= limit {
            words.sort();
            return words;
        }
        if edit_distance(&k, word) <= 1 || k.starts_with(word) {
            words.push(k.to_owned());
        }
    }
    words.sort();
    words
}

pub fn edit_distance(a: &str, b: &str) -> usize {
    #[derive(PartialEq, Eq, Hash)]
    struct Location(usize, usize);

    let mut m: HashMap<Location, usize> = HashMap::new();
    fn inner(a: &[u8], b: &[u8], i: usize, j: usize, m: &mut HashMap<Location, usize>) -> usize {
        if let Some(n) = m.get(&Location(i, j)) {
            return *n;
        }

        let n = if i == 0 || j == 0 {
            max(i, j)
        } else if a[i - 1] == b[j - 1] {
            inner(a, b, i - 1, j - 1, m)
        } else {
            let n1 = inner(a, b, i - 1, j, m);
            let n2 = inner(a, b, i, j - 1, m);
            let n3 = inner(a, b, i - 1, j - 1, m);
            1 + min(min(n1, n2), n3)
        };
        m.insert(Location(i, j), n);
        n
    }
    inner(a.as_bytes(), b.as_bytes(), a.len(), b.len(), &mut m)
}

async fn lookup(
    Path(word): Path<String>,
    Extension(state): Extension<Arc<State>>,
) -> impl IntoResponse {
    let dict_file = state.dict_file;
    let dict = match load_dictionary(dict_file) {
        Err(e) => {
            return Json(json!({"status": "error", "msg": e.to_string()}));
        }
        Ok(words) => words,
    };
    let word = word.to_ascii_lowercase();
    let result = dict.get(word.as_str()).cloned();
    match result {
        None => Json(json!({"status": "error", "msg": "the word is not found"})),
        Some(definations) => Json(json!(definations)),
    }
}

struct State {
    dict_file: &'static str,
}

#[tokio::main]
async fn main() {
    let shared_state = Arc::new(State {
        dict_file: "dataset/words.csv",
    });
    let app = Router::new()
        // .route("/words", get(all_wrods))
        .route("/words/:word", get(lookup).layer(Extension(shared_state)));
    let addr = SocketAddr::from_str("127.0.0.1:3000").unwrap();
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
