use colored_json;

use colored_json::prelude::*;

fn main() -> Result<(), Box<dyn ::std::error::Error>> {
    println!(
        "{}",
        r#"{
              "array": [
                "ele1",
                "ele2"
              ],
              "float": 3.1415926,
              "integer": 4398798674962568,
              "string": "string",
              "null": null
           }
        "#
        .to_colored_json_auto()?
    );
    Ok(())
}
