(component
 (core module $m
  (memory (export "memory") 1)
  (data (i32.const 0) "\00\04\00\00\6b\00\00\00")
  (data (i32.const 16) "\00\00\00\00\00\08\00\00\0b\00\00\00")
  (data (i32.const 1024) "\7b\22\74\6f\6f\6c\73\22\3a\5b\7b\22\6e\61\6d\65\22\3a\22\61\63\6b\22\2c\22\64\65\73\63\72\69\70\74\69\6f\6e\22\3a\22\52\65\74\75\72\6e\20\61\20\66\69\78\74\75\72\65\20\61\63\6b\6e\6f\77\6c\65\64\67\65\6d\65\6e\74\22\2c\22\69\6e\70\75\74\53\63\68\65\6d\61\22\3a\7b\22\74\79\70\65\22\3a\22\6f\62\6a\65\63\74\22\7d\7d\5d\7d")
  (data (i32.const 2048) "\7b\22\6f\6b\22\3a\74\72\75\65\7d")
  (func (export "describe") (result i32) (i32.const 0))
  (func (export "invoke") (param i32 i32 i32 i32) (result i32) (i32.const 16))
  (func (export "realloc") (param i32 i32 i32 i32) (result i32) (i32.const 4096)))
 (core instance $i (instantiate $m))
 (func (export "describe") (result string) (canon lift (core func $i "describe") (memory $i "memory")))
 (func (export "invoke") (param "method" string) (param "input" string) (result (result string (error string)))
  (canon lift (core func $i "invoke") (memory $i "memory") (realloc (func $i "realloc")))))
