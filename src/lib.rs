//! SIMD-accelerated PDF parser, reader, and writer.

#![feature(try_blocks)]

// For the reference used in this project, see:
// https://www.adobe.com/content/dam/acom/en/devnet/pdf/pdfs/pdf_reference_1-7.pdf

// For the example PDF file used to figure out the format, see:
// https://wwwimages2.adobe.com/content/dam/cc/en/legal/servicetou/Stock-Additional-Terms-en_US-20200615.pdf

// Every PDF file starts with a single-line header, that for PDF 1.7 files should
// be `%PDF-1.7`. If PDF file contains binary data, it should be followed by a
// comment line with at least four binary characters, i.e. bytes >= 128.

// The last line of a PDF file contains the end-of-file marker `%%EOF`.

// Each PDF file needs an xref table, which holds references to “indirect objects”
// in the file, expressed using byte offsets from the start of the file.

// %PDF-1.7
// %<80><81><82><83>
// startxref
// .. offset ..
// %%EOF

pub struct Writer<'a> {
	stream: &'a mut dyn std::io::Write,
	offset: usize,
}

impl<'a> Writer<'a> {
	/// Creates new writer from the given stream.
	pub fn new(stream: &'a mut dyn std::io::Write) -> Self {
		return Self {
			stream,
			offset: 0,
		};
	}
}

impl Writer<'_> {
	/// Attempts to write the entire buffer into the underlying stream.
	#[must_use]
	pub fn write(&mut self, buf: &[u8]) -> std::io::Result<()> {
		self.stream.write_all(buf)?;
		// Only advance the offset if we wrote everything successfully.
		self.offset += buf.len();
		return Ok(());
	}

	/// Gets the current position in the stream.
	#[inline]
	pub fn pos(&self) -> usize {
		return self.offset;
	}
}

pub struct Document;

impl Document {
	/// Attempts to write the entire document using the provided [`Writer`].
	pub fn write(&mut self, writer: &mut Writer<'_>) -> std::io::Result<()> {
		const NEWLINE: u8 = 0x0a;
		const PERCENT: u8 = 0x25;

		// Write PDF header.
		writer.write(b"%PDF-1.7\n")?;
		// Mark the file as containing binary data, because we want to be able to
		// embed fonts and images.
		writer.write(&[PERCENT, 0x80, 0x81, 0x82, 0x83, NEWLINE])?;

		let catalog_pos = writer.pos();
		writer.write(b"1 0 obj\n")?;
		writer.write(b"<<\n")?;
		writer.write(b"/Type /Catalog\n")?;
		writer.write(b"/Pages 2 0 R\n")?;
		writer.write(b">>\n")?;
		writer.write(b"endobj\n")?;

		let page_tree_pos = writer.pos();
		writer.write(b"2 0 obj\n")?;
		writer.write(b"<<\n")?;
		writer.write(b"/Type /Pages\n")?;
		writer.write(b"/Kids [3 0 R]\n")?;
		writer.write(b"/Count 1\n")?;
		writer.write(b">>\n")?;
		writer.write(b"endobj\n")?;

		let page1_pos = writer.pos();
		writer.write(b"3 0 obj\n")?;
		writer.write(b"<<\n")?;
		writer.write(b"/Type /Page\n")?;
		writer.write(b"/Parent 2 0 R\n")?;
		writer.write(b"/MediaBox [0.0 0.0 612.0 792.0]\n")?;
		writer.write(b"/Contents 4 0 R\n")?;
		writer.write(b"/Resources << /Font << /F13 5 0 R >> >>\n")?;
		writer.write(b">>\n")?;
		writer.write(b"endobj\n")?;

		// Small overview of the text rendering facilities in PDF 1.7:
		//   Each rendered text needs to be a stream object. The stream starts with
		// command "BT", which stands for "Begin Text", and ends with command "ET",
		// for "End Text". Between these you select font and size using "Tf" command,
		// which accepts font name ("/F13") and size in points/units. Text position
		// is provided using "Td" command, which accepts offset from the "0,0" point
		// (lower left corner) in units. "Tj" command draws the text provided as a
		// string using the currently set options (colour, size, font, position.)

		let page1_contents_pos = writer.pos();
		writer.write(b"4 0 obj\n")?;
		writer.write(b"<<\n")?;
		// TODO: How can we get that length programatically?
		writer.write(b"/Length 39\n")?;
		writer.write(b">>\n")?;
		writer.write(b"stream\n")?;
		writer.write(b"BT\n")?;
		writer.write(b"/F13 10 Tf\n")?;
		writer.write(b"12 775 Td\n")?;
		writer.write(b"(Hello!) Tj\n")?;
		writer.write(b"ET\n")?;
		writer.write(b"endstream\n")?;
		writer.write(b"endobj\n")?;

		// TODO: We want to use our own fonts instead of the built-in ones... That
		// would probably require compressing them using DEFLATE, which is going to
		// be a pain in Rust, since I don't want to pull in entire huge dependency
		// just for that... I would rather implement it myself.
		let font_pos = writer.pos();
		writer.write(b"5 0 obj\n")?;
		writer.write(b"<<\n")?;
		writer.write(b"/Type /Font\n")?;
		writer.write(b"/Subtype /Type1\n")?;
		writer.write(b"/Name /F13\n")?;
		writer.write(b"/BaseFont /Times-Roman\n")?;
		writer.write(b"/Encoding /MacRomanEncoding\n")?;
		writer.write(b">>\n")?;
		writer.write(b"endobj\n")?;

		let xref_pos = writer.pos();
		writer.write(b"xref\n")?;
		writer.write(b"0 6\n")?;
		std::write!(writer.stream, "0000000000 65535 f\r\n")?;
		std::write!(writer.stream, "{:0>10} 00000 n\r\n", catalog_pos)?;
		std::write!(writer.stream, "{:0>10} 00000 n\r\n", page_tree_pos)?;
		std::write!(writer.stream, "{:0>10} 00000 n\r\n", page1_pos)?;
		std::write!(writer.stream, "{:0>10} 00000 n\r\n", page1_contents_pos)?;
		std::write!(writer.stream, "{:0>10} 00000 n\r\n", font_pos)?;

		// Write the document trailer.
		writer.write(b"trailer\n")?;
		writer.write(b"<<\n")?;
		writer.write(b"/Size 6\n")?;
		writer.write(b"/Root 1 0 R\n")?;
		writer.write(b">>\n")?;

		writer.write(b"startxref\n")?;
		std::write!(writer.stream, "{}\n", xref_pos)?;
		writer.write(b"%%EOF\n")?;

		return Ok(());
	}
}

#[cfg(test)]
mod tests {
	#[test]
	fn it_works() {
		let mut file = std::fs::File::create("sample.pdf").expect("failed to create a file");
		let mut writer = crate::Writer::new(&mut file);

		let mut doc = crate::Document;
		doc.write(&mut writer).expect("failed to write document to file");
	}
}
