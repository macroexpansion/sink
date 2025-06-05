use sink::{SyncClient, SyncNotification};
use std::io::{self, Write};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Simple Text Sync Client Example");
    println!("================================");
    
    // Get client name from user
    print!("Enter your client name: ");
    io::stdout().flush()?;
    let mut client_name = String::new();
    io::stdin().read_line(&mut client_name)?;
    let client_name = client_name.trim().to_string();
    
    // Connect to server
    let mut client = SyncClient::new(client_name.clone());
    println!("Connecting to server...");
    
    match client.connect("ws://127.0.0.1:8080").await {
        Ok(_) => println!("✓ Connected successfully!"),
        Err(e) => {
            println!("✗ Failed to connect: {}", e);
            println!("Make sure the server is running with: cargo run server");
            return Ok(());
        }
    }
    
    // Create or use existing document
    println!("\nDocument Operations:");
    println!("1. Create new document");
    println!("2. Use existing document");
    print!("Choose option (1 or 2): ");
    io::stdout().flush()?;
    
    let mut choice = String::new();
    io::stdin().read_line(&mut choice)?;
    
    let document_id = match choice.trim() {
        "1" => {
            print!("Enter document name: ");
            io::stdout().flush()?;
            let mut doc_name = String::new();
            io::stdin().read_line(&mut doc_name)?;
            let doc_name = doc_name.trim().to_string();
            
            match client.create_document(doc_name.clone()).await {
                Ok(id) => {
                    println!("✓ Created document '{}' with ID: {}", doc_name, id);
                    id
                }
                Err(e) => {
                    println!("✗ Failed to create document: {}", e);
                    return Ok(());
                }
            }
        }
        "2" => {
            // List existing documents
            match client.list_documents().await {
                Ok(documents) => {
                    if documents.is_empty() {
                        println!("No documents available. Creating a default one...");
                        client.create_document("shared.txt".to_string()).await?
                    } else {
                        println!("Available documents:");
                        for (i, doc) in documents.iter().enumerate() {
                            println!("  {}. {} (ID: {}, {} bytes)", 
                                    i + 1, doc.name, doc.id, doc.content_length);
                        }
                        
                        print!("Choose document number: ");
                        io::stdout().flush()?;
                        let mut doc_choice = String::new();
                        io::stdin().read_line(&mut doc_choice)?;
                        
                        let doc_index: usize = doc_choice.trim().parse().unwrap_or(1) - 1;
                        if doc_index < documents.len() {
                            documents[doc_index].id
                        } else {
                            println!("Invalid choice, using first document");
                            documents[0].id
                        }
                    }
                }
                Err(e) => {
                    println!("✗ Failed to list documents: {}", e);
                    return Ok(());
                }
            }
        }
        _ => {
            println!("Invalid choice, creating default document");
            client.create_document("default.txt".to_string()).await?
        }
    };
    
    // Get current document content
    match client.get_document(document_id).await {
        Ok((content, last_modified)) => {
            println!("\nCurrent document content:");
            println!("'{}'", content);
            println!("Last modified: {}", last_modified);
        }
        Err(e) => {
            println!("✗ Failed to get document: {}", e);
        }
    }
    
    // Interactive text editing
    println!("\n=== Interactive Text Editor ===");
    println!("Type text to update the document. Type 'quit' to exit.");
    println!("Your changes will be synchronized with other clients in real-time.");
    
    // Start a background task to listen for notifications
    let notification_client_name = client_name.clone();
    tokio::spawn(async move {
        // This is a simplified version - in a real implementation you'd need
        // to properly handle the client connection for notifications
        loop {
            sleep(Duration::from_secs(1)).await;
        }
    });
    
    // Main input loop
    loop {
        print!("\n> ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input == "quit" {
            break;
        }
        
        if input.is_empty() {
            continue;
        }
        
        // Update document with new content
        match client.update_document(document_id, input.to_string()).await {
            Ok(_) => {
                println!("✓ Document updated successfully");
                
                // Get and display updated content
                if let Ok((content, _)) = client.get_document(document_id).await {
                    println!("Current content: '{}'", content);
                }
            }
            Err(e) => {
                println!("✗ Failed to update document: {}", e);
            }
        }
    }
    
    println!("Goodbye!");
    Ok(())
}
