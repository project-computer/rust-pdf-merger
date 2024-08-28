use std::collections::{HashMap, BTreeMap};
use lopdf::{Document, Object, ObjectId, dictionary, Stream, Bookmark};
use lopdf::content::{Content, Operation};

pub fn merge_documents(pdf_pages: HashMap<String, Vec<u32>>) -> Result<Document, lopdf::Error> {
    let mut max_id = 1;
    let mut pagenum = 1;
    let mut documents_pages = BTreeMap::new();
    let mut documents_objects = BTreeMap::new();
    let mut document = Document::with_version("1.5");
    for (path, page_numbers) in pdf_pages {
        let mut doc = Document::load(path)?;
        let mut first = true;
        doc.renumber_objects_with(max_id);
        
        max_id = doc.max_id + 1;

        let pages: BTreeMap<u32, (u32, u16)> = doc.get_pages();
        for (page_number, (object_id, generation)) in pages.iter() {
            println!("Page Number: {}, Object ID: {}, Generation: {}", page_number, object_id, generation);
        }
        for &page_number in &page_numbers {
            println!("{}", page_number);
            if let Some(&object_id) = pages.get(&page_number) {
                if first {
                    let bookmark = Bookmark::new(
                        format!("Page_{}", pagenum),
                        [0.0, 0.0, 1.0],
                        0,
                        object_id
                    );
                    document.add_bookmark(bookmark, None);
                    first = false;
                    pagenum += 1;
                }

                documents_pages.insert(
                    object_id,
                    doc.get_object(object_id)?.to_owned(),
                );
            }
        }
        documents_objects.extend(doc.objects);
    }
    
    // "Catalog" and "Pages" are mandatory
    let mut catalog_object: Option<(ObjectId, Object)> = None;
    let mut pages_object: Option<(ObjectId, Object)> = None;

    // Process all objects except "Page" type
    for (object_id, object) in documents_objects.iter() {
        // We have to ignore "Page" (as are processed later), "Outlines" and "Outline" objects.
        // All other objects should be collected and inserted into the main Document.
        match object.type_name().unwrap_or("") {
            "Catalog" => {
                // Collect a first "Catalog" object and use it for the future "Pages".
                catalog_object = Some((
                    if let Some((id, _)) = catalog_object {
                        id
                    } else {
                        *object_id
                    },
                    object.clone(),
                ));
            }
            "Pages" => {
                // Collect and update a first "Pages" object and use it for the future "Catalog"
                // We have also to merge all dictionaries of the old and the new "Pages" object
                if let Ok(dictionary) = object.as_dict() {
                    let mut dictionary = dictionary.clone();
                    if let Some((_, ref object)) = pages_object {
                        if let Ok(old_dictionary) = object.as_dict() {
                            dictionary.extend(old_dictionary);
                        }
                    }

                    pages_object = Some((
                        if let Some((id, _)) = pages_object {
                            id
                        } else {
                            *object_id
                        },
                        Object::Dictionary(dictionary),
                    ));
                }
            }
            "Page" => {}     // Ignored, processed later and separately
            "Outlines" => {} // Ignored, not supported yet
            "Outline" => {}  // Ignored, not supported yet
            _ => {
                document.objects.insert(*object_id, object.clone());
            }
        }
    }

    // If no "Pages" object found, return an error
    if pages_object.is_none() {
        println!("Pages root not found.");

        return Ok(document);
    }
    // Iterate over all "Page" objects and collect into the parent "Pages" created before
    for (object_id, object) in documents_pages.iter() {
        if let Ok(dictionary) = object.as_dict() {
            let mut dictionary = dictionary.clone();
            dictionary.set("Parent", pages_object.as_ref().unwrap().0);
            document.objects.insert(*object_id, Object::Dictionary(dictionary));
        }
    }

    // If no "Catalog" found, return an error
     if catalog_object.is_none() {
        println!("Catalog root not found.");

        return Ok(document);
    }



    let catalog_object = catalog_object.unwrap();
    let pages_object = pages_object.unwrap();
    // Build a new "Pages" with updated fields
    if let Ok(dictionary) = pages_object.1.as_dict() {
        let mut dictionary = dictionary.clone();
        dictionary.set("Count", documents_pages.len() as u32);
        dictionary.set(
            "Kids",
            documents_pages
                .into_iter()
                .map(|(object_id, _)| Object::Reference(object_id))
                .collect::<Vec<_>>(),
        );
        document.objects.insert(pages_object.0, Object::Dictionary(dictionary));
    }

    // Build a new "Catalog" with updated fields
    if let Ok(dictionary) = catalog_object.1.as_dict() {
        let mut dictionary = dictionary.clone();
        dictionary.set("Pages", pages_object.0);
        dictionary.remove(b"Outlines"); // Outlines not supported in merged PDFs
        document.objects.insert(catalog_object.0, Object::Dictionary(dictionary));
    }

    document.trailer.set("Root", catalog_object.0);

    // Update the max internal ID as wasn't updated before due to direct objects insertion
    document.max_id = document.objects.len() as u32;

    // Reorder all new Document objects
    document.renumber_objects();
    println!("docun{}",document.page_iter().count());
    // Set any Bookmarks to the First child if they are not set to a page
    document.adjust_zero_pages();

    // Set all bookmarks to the PDF Object tree then set the Outlines to the Bookmark content map.
    if let Some(n) = document.build_outline() {
        if let Ok(x) = document.get_object_mut(catalog_object.0) {
            if let Object::Dictionary(ref mut dict) = x {
                dict.set("Outlines", Object::Reference(n));
            }
        }
    }

    document.compress();

    Ok(document)
}