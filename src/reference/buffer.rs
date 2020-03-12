use std::ops::Range;
use pct_str::PctStr;
use crate::parsing::{self, ParsedIriRef};
use crate::{Error, Authority, AuthorityMut, Path, PathMut};
use super::IriRef;

/// Owned IRI reference.
pub struct IriRefBuf {
	pub(crate) p: ParsedIriRef,
	pub(crate) data: Vec<u8>,
}

impl IriRefBuf {
	pub fn new<S: AsRef<[u8]> + ?Sized>(buffer: &S) -> Result<IriRefBuf, Error> {
		Ok(IriRefBuf {
			data: Vec::from(buffer.as_ref()),
			p: ParsedIriRef::new(buffer)?
		})
	}

	pub fn as_iri_ref(&self) -> IriRef {
		IriRef {
			data: self.data.as_ref(),
			p: self.p
		}
	}

	/// Length in bytes.
	pub fn len(&self) -> usize {
		self.p.len()
	}

	pub fn as_str(&self) -> &str {
		unsafe {
			std::str::from_utf8_unchecked(&self.data[0..self.len()])
		}
	}

	pub fn as_pct_str(&self) -> &PctStr {
		unsafe {
			PctStr::new_unchecked(self.as_str())
		}
	}

	pub fn scheme(&self) -> Option<&PctStr> {
		if let Some(scheme_len) = self.p.scheme_len {
			if scheme_len > 0 {
				unsafe {
					Some(PctStr::new_unchecked(std::str::from_utf8_unchecked(&self.data[0..scheme_len])))
				}
			} else {
				None
			}
		} else {
			None
		}
	}

	pub(crate) fn replace(&mut self, range: Range<usize>, content: &[u8]) {
		crate::replace(&mut self.data, &mut self.p.authority, range, content)
	}

	/// Set the scheme of the IRI.
	///
	/// It must be a syntactically correct scheme. If not,
	/// this method returns an error, and the IRI is unchanged.
	pub fn set_raw_scheme<S: AsRef<[u8]> + ?Sized>(&mut self, scheme: Option<&S>) -> Result<(), Error> {
		if scheme.is_none() {
			if let Some(scheme_len) = self.p.scheme_len {
				self.replace(0..(scheme_len+1), &[]);
			}

			self.p.scheme_len = None;
		} else {
			let new_scheme = scheme.unwrap().as_ref();
			let new_scheme_len = parsing::parse_scheme(new_scheme, 0)?;
			if new_scheme_len == 0 || new_scheme_len != new_scheme.len() {
				return Err(Error::Invalid);
			}

			if let Some(scheme_len) = self.p.scheme_len {
				self.replace(0..scheme_len, new_scheme);
			} else {
				self.replace(0..0, &[0x3a]);
				self.replace(0..0, new_scheme);
			}

			self.p.scheme_len = Some(new_scheme_len);
		}

		Ok(())
	}

	/// Set the scheme of the IRI.
	///
	/// It must be a syntactically correct scheme. If not,
	/// this method returns an error, and the IRI is unchanged.
	pub fn set_scheme(&mut self, scheme: Option<&str>) -> Result<(), Error> {
		self.set_raw_scheme(scheme)
	}

	pub fn authority(&self) -> Authority {
		Authority {
			data: self.data.as_ref(),
			authority: &self.p.authority
		}
	}

	pub fn authority_mut(&mut self) -> AuthorityMut {
		AuthorityMut {
			data: &mut self.data,
			authority: &mut self.p.authority
		}
	}

	/// Set the authority of the IRI.
	///
	/// It must be a syntactically correct authority. If not,
	/// this method returns an error, and the IRI is unchanged.
	pub fn set_authority<S: AsRef<[u8]> + ?Sized>(&mut self, authority: &S) -> Result<(), Error> {
		let new_authority = authority.as_ref();
		let mut new_parsed_authority = parsing::parse_authority(new_authority, 0)?;
		if new_parsed_authority.len() != new_authority.len() {
			return Err(Error::Invalid);
		}
		let offset = self.p.authority.offset;
		new_parsed_authority.offset = offset;
		self.replace(offset..(offset+self.p.authority.len()), new_authority);
		self.p.authority = new_parsed_authority;
		Ok(())
	}

	pub fn path<'a>(&'a self) -> Path<'a> {
		let offset = self.p.authority.offset + self.p.authority.len();
		Path {
			data: &self.data[offset..(offset+self.p.path_len)]
		}
	}

	pub fn path_mut<'a>(&'a mut self) -> PathMut<'a> {
		PathMut {
			buffer: self
		}
	}

	pub fn set_path<S: AsRef<[u8]> + ?Sized>(&mut self, path: &S) -> Result<(), Error> {
		let new_path = path.as_ref();
		let new_path_len = parsing::parse_path(new_path, 0)?;
		if new_path_len != new_path.len() {
			return Err(Error::Invalid);
		}
		let offset = self.p.path_offset();
		self.replace(offset..(offset+self.p.path_len), new_path);
		self.p.path_len = new_path_len;
		Ok(())
	}

	pub fn query(&self) -> Option<&PctStr> {
		if let Some(len) = self.p.query_len {
			if len > 0 {
				unsafe {
					let offset = self.p.query_offset();
					Some(PctStr::new_unchecked(std::str::from_utf8_unchecked(&self.data[offset..(offset+len)])))
				}
			} else {
				None
			}
		} else {
			None
		}
	}

	pub fn set_raw_query<S: AsRef<[u8]> + ?Sized>(&mut self, query: Option<&S>) -> Result<(), Error> {
		let offset = self.p.query_offset();

		if query.is_none() || query.unwrap().as_ref().is_empty() {
			if let Some(query_len) = self.p.query_len {
				self.replace((offset-1)..(offset+query_len), &[]);
			}

			self.p.query_len = None;
		} else {
			let new_query = query.unwrap().as_ref();
			let new_query_len = parsing::parse_query(new_query, 0)?;
			if new_query_len != new_query.len() {
				return Err(Error::Invalid);
			}

			if let Some(query_len) = self.p.query_len {
				self.replace(offset..(offset+query_len), new_query);
			} else {
				self.replace(offset..offset, &[0x3f]);
				self.replace((offset+1)..(offset+1), new_query);
			}

			self.p.query_len = Some(new_query_len);
		}

		Ok(())
	}

	pub fn set_query(&mut self, query: Option<&str>) -> Result<(), Error> {
		self.set_raw_query(query)
	}

	pub fn fragment(&self) -> Option<&PctStr> {
		if let Some(len) = self.p.fragment_len {
			if len > 0 {
				unsafe {
					let offset = self.p.fragment_offset();
					Some(PctStr::new_unchecked(std::str::from_utf8_unchecked(&self.data[offset..(offset+len)])))
				}
			} else {
				None
			}
		} else {
			None
		}
	}

	pub fn set_raw_fragment<S: AsRef<[u8]> + ?Sized>(&mut self, fragment: Option<&S>) -> Result<(), Error> {
		let offset = self.p.fragment_offset();

		if fragment.is_none() || fragment.unwrap().as_ref().is_empty() {
			if let Some(fragment_len) = self.p.fragment_len {
				self.replace((offset-1)..(offset+fragment_len), &[]);
			}

			self.p.fragment_len = None;
		} else {
			let new_fragment = fragment.unwrap().as_ref();
			let new_fragment_len = parsing::parse_fragment(new_fragment, 0)?;
			if new_fragment_len != new_fragment.len() {
				return Err(Error::Invalid);
			}

			if let Some(fragment_len) = self.p.fragment_len {
				self.replace(offset..(offset+fragment_len), new_fragment);
			} else {
				self.replace(offset..offset, &[0x23]);
				self.replace((offset+1)..(offset+1), new_fragment);
			}

			self.p.fragment_len = Some(new_fragment_len);
		}

		Ok(())
	}

	pub fn set_fragment(&mut self, fragment: Option<&str>) -> Result<(), Error> {
		self.set_raw_fragment(fragment)
	}
}