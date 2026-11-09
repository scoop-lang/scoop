; Only this native companion enables non-canonical compact personalities.
; Scoop must retain DWARF EH while consuming the resulting fixed archive.
@_ZTIi = external constant ptr

declare void @native_throw() noreturn
declare i32 @__gxx_personality_v0(...)
declare i32 @native_personality_two(...)
declare i32 @native_personality_three(...)
declare ptr @__cxa_begin_catch(ptr) nounwind
declare void @__cxa_end_catch() nounwind

define i32 @native_probe1() noinline uwtable(sync) "frame-pointer"="all" personality ptr @__gxx_personality_v0 {
entry:
  invoke void @native_throw() to label %unreachable unwind label %catch

unreachable:
  unreachable

catch:
  %caught = landingpad { ptr, i32 } catch ptr @_ZTIi
  %exception = extractvalue { ptr, i32 } %caught, 0
  %pointer = call ptr @__cxa_begin_catch(ptr %exception)
  %value = load i32, ptr %pointer, align 4
  call void @__cxa_end_catch()
  %result = add i32 %value, 1
  ret i32 %result
}

define i32 @native_probe2() noinline uwtable(sync) "frame-pointer"="all" personality ptr @native_personality_two {
entry:
  invoke void @native_throw() to label %unreachable unwind label %catch

unreachable:
  unreachable

catch:
  %caught = landingpad { ptr, i32 } catch ptr @_ZTIi
  %exception = extractvalue { ptr, i32 } %caught, 0
  %pointer = call ptr @__cxa_begin_catch(ptr %exception)
  %value = load i32, ptr %pointer, align 4
  call void @__cxa_end_catch()
  %result = add i32 %value, 1
  ret i32 %result
}

define i32 @native_probe3() noinline uwtable(sync) "frame-pointer"="all" personality ptr @native_personality_three {
entry:
  invoke void @native_throw() to label %unreachable unwind label %catch

unreachable:
  unreachable

catch:
  %caught = landingpad { ptr, i32 } catch ptr @_ZTIi
  %exception = extractvalue { ptr, i32 } %caught, 0
  %pointer = call ptr @__cxa_begin_catch(ptr %exception)
  %value = load i32, ptr %pointer, align 4
  call void @__cxa_end_catch()
  %result = add i32 %value, 1
  ret i32 %result
}
