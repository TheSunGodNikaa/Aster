use eframe::egui::{self, Align, Color32, Id, Layout, Margin, RichText, ScrollArea, Stroke, vec2};
use crate::{app::AsterApp, database::{ProductRecord, PurchaseDraft, PurchaseLineDraft, PurchaseSummary, SupplierRecord}};

const INK: Color32 = Color32::from_rgb(43,39,52);
const MUTED: Color32 = Color32::from_rgb(100,94,111);
const PURPLE: Color32 = Color32::from_rgb(91,65,139);
const BORDER: Color32 = Color32::from_rgb(226,223,232);
const GREEN: Color32 = Color32::from_rgb(38,110,72);
const RED: Color32 = Color32::from_rgb(176,58,58);

#[derive(Clone, Debug)]
struct LineForm { product_id: i64, batch_no: String, expiry_date: String, qty: String, free_qty: String, rate: String, discount: String, mrp: String, sale_rate: String }
impl Default for LineForm { fn default()->Self { Self{product_id:0,batch_no:String::new(),expiry_date:String::new(),qty:"1".into(),free_qty:"0".into(),rate:"0.00".into(),discount:"0".into(),mrp:"0.00".into(),sale_rate:"0.00".into()} } }
#[derive(Clone, Default)]
struct State { loaded:bool, suppliers:Vec<SupplierRecord>, products:Vec<ProductRecord>, recent:Vec<PurchaseSummary>, supplier_id:i64, supplier_invoice_no:String, invoice_date:String, notes:String, lines:Vec<LineForm>, message:String, error:bool, search_product:String }
impl State { fn reset(&mut self){self.supplier_id=0;self.supplier_invoice_no.clear();self.invoice_date=crate::date::local_date();self.notes.clear();self.lines=vec![LineForm::default()];self.message.clear();self.error=false;} }
pub(crate) fn show(ui:&mut egui::Ui, app:&mut AsterApp){
 let key=Id::new(("purchase-entry",app.selected_year)); let mut s=ui.ctx().data_mut(|d|d.get_temp::<State>(key).unwrap_or_default());
 if !s.loaded { s.loaded=true; s.reset(); reload(&mut s,app); }
 ui.horizontal(|ui|{ui.vertical(|ui|{ui.label(RichText::new("TRANSACTIONS  /  INVENTORY INWARD").size(10.5).strong().color(PURPLE));ui.heading(RichText::new("Purchase Entry").size(28.0).color(INK));ui.label(RichText::new("Record supplier invoices, batches, expiry and incoming stock in one transaction.").size(12.5).color(MUTED));});ui.with_layout(Layout::right_to_left(Align::Center),|ui|{if ui.button("Refresh").clicked(){reload(&mut s,app)}if ui.button("+ Add line").clicked(){s.lines.push(LineForm::default());}});});
 ui.add_space(12.0);
 if s.suppliers.is_empty(){ui.colored_label(RED,"No active suppliers found. Create or reactivate a supplier first.");}
 egui::Frame::new().fill(Color32::WHITE).stroke(Stroke::new(1.0,BORDER)).corner_radius(12.0).inner_margin(Margin::same(14)).show(ui,|ui|{
   ui.heading(RichText::new("Supplier invoice").size(16.0).color(INK));ui.add_space(8.0);
   egui::Grid::new("purchase-header-grid").num_columns(2).spacing(vec2(10.0,8.0)).show(ui,|ui|{
     ui.label(RichText::new("Supplier *").color(MUTED)); let selected=s.suppliers.iter().find(|x|x.id==s.supplier_id).map(|x|x.name.clone()).unwrap_or_else(||"Select supplier".into()); egui::ComboBox::from_id_salt("purchase-supplier").selected_text(selected).width(300.0).show_ui(ui,|ui|{for x in &s.suppliers{ui.selectable_value(&mut s.supplier_id,x.id,format!("{}{}",x.name,if x.gstin.is_empty(){String::new()}else{format!(" · {}",x.gstin)}));}});ui.end_row();
     ui.label(RichText::new("Supplier invoice no. *").color(MUTED));ui.add(egui::TextEdit::singleline(&mut s.supplier_invoice_no).hint_text("As printed on supplier invoice").desired_width(300.0));ui.end_row();
     ui.label(RichText::new("Invoice date *").color(MUTED));ui.add(egui::TextEdit::singleline(&mut s.invoice_date).hint_text("YYYY-MM-DD").desired_width(140.0));ui.end_row();
     ui.label(RichText::new("Notes").color(MUTED));ui.add(egui::TextEdit::singleline(&mut s.notes).desired_width(300.0));ui.end_row();
   });
 });
 ui.add_space(12.0);
 egui::Frame::new().fill(Color32::WHITE).stroke(Stroke::new(1.0,BORDER)).corner_radius(12.0).inner_margin(Margin::same(14)).show(ui,|ui|{
  ui.horizontal(|ui|{ui.heading(RichText::new("Purchase line items").size(16.0).color(INK));ui.with_layout(Layout::right_to_left(Align::Center),|ui|{ui.add(egui::TextEdit::singleline(&mut s.search_product).hint_text("Filter products").desired_width(210.0));});});
  ui.label(RichText::new("Quantities support up to three decimal places. Rates and totals are stored in paise; each line snapshots product, HSN and GST.").size(11.0).color(MUTED));ui.add_space(8.0);
  ScrollArea::horizontal().id_salt("purchase-lines-horizontal").auto_shrink([false,false]).show(ui,|ui|{
   egui::Grid::new("purchase-lines-grid").striped(true).min_col_width(70.0).spacing(vec2(8.0,7.0)).show(ui,|ui|{
    for h in ["Product *","Batch *","Expiry *","Qty *","Free qty","Rate ₹","Disc %","MRP ₹","Sale ₹","Remove"]{ui.label(RichText::new(h).strong().size(10.5).color(PURPLE));}ui.end_row();
    let filtered:Vec<_>=s.products.iter().filter(|p|s.search_product.trim().is_empty()||format!("{} {} {}",p.name,p.brand_name,p.generic_name).to_lowercase().contains(&s.search_product.to_lowercase())).cloned().collect();
    let mut remove=None;
    for (i,line) in s.lines.iter_mut().enumerate(){
      let current=s.products.iter().find(|p|p.id==line.product_id).map(|p|format!("{}{}",p.name,if p.brand_name.is_empty(){String::new()}else{format!(" · {}",p.brand_name)})).unwrap_or_else(||"Select product".into());
      egui::ComboBox::from_id_salt(("purchase-product",i)).selected_text(current).width(210.0).show_ui(ui,|ui|{for p in &filtered{ui.selectable_value(&mut line.product_id,p.id,format!("{}{}",p.name,if p.manufacturer_name.is_empty(){String::new()}else{format!(" · {}",p.manufacturer_name)}));}});
      ui.add(egui::TextEdit::singleline(&mut line.batch_no).desired_width(100.0).hint_text("Batch no"));ui.add(egui::TextEdit::singleline(&mut line.expiry_date).desired_width(100.0).hint_text("YYYY-MM-DD"));ui.add(egui::TextEdit::singleline(&mut line.qty).desired_width(65.0));ui.add(egui::TextEdit::singleline(&mut line.free_qty).desired_width(65.0));ui.add(egui::TextEdit::singleline(&mut line.rate).desired_width(80.0));ui.add(egui::TextEdit::singleline(&mut line.discount).desired_width(65.0));ui.add(egui::TextEdit::singleline(&mut line.mrp).desired_width(80.0));ui.add(egui::TextEdit::singleline(&mut line.sale_rate).desired_width(80.0));if ui.small_button("×").clicked(){remove=Some(i)}ui.end_row();
    }
    if let Some(i)=remove{if s.lines.len()>1{s.lines.remove(i);}else{s.lines[0]=LineForm::default();}}
   });
  });
 });
 let totals=preview_totals(&s);
 ui.add_space(10.0);ui.horizontal(|ui|{ui.label(RichText::new(format!("Estimated taxable: {}",money(totals.0))).strong().color(INK));ui.separator();ui.label(RichText::new(format!("Estimated GST: {}",money(totals.1))).color(MUTED));ui.separator();ui.label(RichText::new(format!("Estimated total: {}",money(totals.0+totals.1))).size(18.0).strong().color(PURPLE));});
 if !s.message.is_empty(){ui.add_space(5.0);ui.colored_label(if s.error{RED}else{GREEN},&s.message);}
 ui.add_space(8.0);ui.horizontal(|ui|{if ui.button(RichText::new("Post purchase & update stock").strong()).clicked(){post(&mut s,app);}if ui.button("Clear form").clicked(){s.reset();}});
 ui.add_space(18.0);ui.heading(RichText::new("Recent purchases").size(18.0).color(INK));ui.add_space(6.0);
 egui::Frame::new().fill(Color32::WHITE).stroke(Stroke::new(1.0,BORDER)).corner_radius(10.0).inner_margin(Margin::same(10)).show(ui,|ui|{egui::Grid::new("recent-purchases-grid").striped(true).num_columns(5).spacing(vec2(14.0,7.0)).show(ui,|ui|{for h in ["Purchase no.","Date","Supplier","Supplier invoice","Total"]{ui.label(RichText::new(h).strong().color(PURPLE));}ui.end_row();if s.recent.is_empty(){ui.label("No posted purchases yet.");ui.end_row();}for r in &s.recent{ui.label(&r.purchase_no);ui.label(&r.invoice_date);ui.label(&r.supplier_name);ui.label(&r.supplier_invoice_no);ui.label(money(r.total_paise));ui.end_row();}})});
 ui.ctx().data_mut(|d|d.insert_temp(key,s));
}
fn parse_money(v:&str)->Result<i64,String>{let n=if v.trim().is_empty(){0.0}else{v.trim().replace(',','').parse::<f64>().map_err(|_|format!("Invalid money amount: {v}"))?};if !n.is_finite()||n<0.0||n>(i64::MAX/100) as f64{return Err("Money amount is invalid or out of range.".into())}Ok((n*100.0).round() as i64)}
fn parse_qty(v:&str,label:&str,allow_zero:bool)->Result<i64,String>{let n=if v.trim().is_empty(){0.0}else{v.trim().parse::<f64>().map_err(|_|format!("{label} must be a number."))?};if !n.is_finite()||n<0.0||(!allow_zero&&n<=0.0)||n>9_000_000_000.0{return Err(format!("{label} is outside the allowed range."))}let milli=(n*1000.0).round();if (n*1000.0-milli).abs()>0.000001{return Err(format!("{label} supports at most three decimal places."))}Ok(milli as i64)}
fn parse_discount(v:&str)->Result<i64,String>{let n=if v.trim().is_empty(){0.0}else{v.trim().parse::<f64>().map_err(|_|"Discount must be a number.".to_owned())?};if !n.is_finite()||!(0.0..=100.0).contains(&n){return Err("Discount must be between 0 and 100%.".into())}Ok((n*100.0).round() as i64)}
fn money(paise:i64)->String{format!("₹{:.2}",paise as f64/100.0)}
fn preview_totals(s:&State)->(i64,i64){let mut taxable=0_i64;let mut tax=0_i64;for l in &s.lines{let (Ok(q),Ok(rate),Ok(discount))=(parse_qty(&l.qty,"Quantity",false),parse_money(&l.rate),parse_discount(&l.discount))else{continue};let Some(p)=s.products.iter().find(|p|p.id==l.product_id)else{continue};let gross=(q as i128*rate as i128/1000) as i64;let base=((gross as i128*(10000-discount) as i128+5000)/10000) as i64;let gst=(p.gst_percent*100.0).round() as i64;taxable=taxable.saturating_add(base);tax=tax.saturating_add(((base as i128*gst as i128+5000)/10000) as i64);} (taxable,tax)}
fn post(s:&mut State,app:&AsterApp){
 let lines:Result<Vec<_>,String>=s.lines.iter().enumerate().map(|(i,l)|{if l.product_id==0{return Err(format!("Line {}: select a product.",i+1));}Ok(PurchaseLineDraft{product_id:l.product_id,batch_no:l.batch_no.trim().into(),expiry_date:l.expiry_date.trim().into(),quantity_milli:parse_qty(&l.qty,"Quantity",false)?,free_quantity_milli:parse_qty(&l.free_qty,"Free quantity",true)?,rate_paise:parse_money(&l.rate)?,discount_basis_points:parse_discount(&l.discount)?,mrp_paise:parse_money(&l.mrp)?,sale_rate_paise:parse_money(&l.sale_rate)?})}).collect();
 let lines=match lines{Ok(v)=>v,Err(e)=>{s.message=e;s.error=true;return;}};
 let draft=PurchaseDraft{supplier_id:s.supplier_id,supplier_invoice_no:s.supplier_invoice_no.trim().into(),invoice_date:s.invoice_date.trim().into(),notes:s.notes.trim().into(),lines};
 let result=app.db.as_ref().ok_or_else(||"Database is not open.".to_owned()).and_then(|db|db.post_purchase(app.selected_year,&draft));
 match result{Ok(no)=>{s.reset();reload(s,app);s.message=format!("Purchase {no} posted. Batch stock and stock movement records were updated atomically.");s.error=false;},Err(e)=>{s.message=e;s.error=true;}}
}
fn reload(s:&mut State,app:&AsterApp){if let Some(db)=app.db.as_ref(){s.suppliers=db.suppliers(app.selected_year,"").unwrap_or_default().into_iter().filter(|x|x.active).collect();s.products=db.products(app.selected_year,"").unwrap_or_default().into_iter().filter(|x|x.active).collect();s.recent=db.recent_purchases(app.selected_year,12).unwrap_or_default();}else{s.suppliers.clear();s.products.clear();s.recent.clear();}}
