use std::collections::HashMap;
mod services;
use services::pdf; // Replace 'your_crate_name' with your actual crate name

fn main() -> Result<(), lopdf::Error> {
    let mut pdf_pages = HashMap::new();
    pdf_pages.insert("pdf-files/temp1.pdf".to_string(), vec![1, 3, 5]);
    pdf_pages.insert("pdf-files/temp2.pdf".to_string(), vec![2, 4]);

    let mut merged_doc = pdf::merge_documents(pdf_pages)?;
    merged_doc.save("merged.pdf")?;

    Ok(())
}