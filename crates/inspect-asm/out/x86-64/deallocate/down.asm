inspect_asm::deallocate::down:
	mov rax, qword ptr [rdi]
	cmp qword ptr [rax], rsi
	je .LBB0_0
	ret
.LBB0_0:
	add rsi, rcx
	mov qword ptr [rax], rsi
	ret
