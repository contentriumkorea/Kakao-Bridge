pub fn authorize(hint:Option<String>,expected:Option<&str>)->Result<Option<String>,&'static str>{
 if let Some(value)=&hint {if Some(value.as_str())!=expected{return Err("EXISTING_TEXT_CONFIRMATION_REQUIRED")}}
 Ok(hint)
}
#[cfg(test)] mod tests {
 use super::*;
 #[test] fn never_replace_existing_hint_without_exact_explicit_acknowledgement(){
  assert_eq!(authorize(None,None),Ok(None));
  assert_eq!(authorize(Some("메시지 입력".into()),None),Err("EXISTING_TEXT_CONFIRMATION_REQUIRED"));
  assert_eq!(authorize(Some("메시지 입력".into()),Some("other")),Err("EXISTING_TEXT_CONFIRMATION_REQUIRED"));
  assert_eq!(authorize(Some("메시지 입력".into()),Some("메시지 입력")),Ok(Some("메시지 입력".into())));
 }
}
