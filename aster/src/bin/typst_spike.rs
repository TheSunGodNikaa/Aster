
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let document = typst_bake::document!("main.typ");

    println!("Pages: {}", document.page_count()?);

    for warning in document.warnings()? {
        eprintln!("Typst warning: {warning}");
    }

    let pdf = document.to_pdf()?;

    println!("PDF size: {} bytes", pdf.len());

    std::fs::write("Docs/typst-spike.pdf", &pdf)?;

    println!("PDF generated successfully.");

    Ok(())
}
