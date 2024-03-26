inspect_asm::deallocate::down:
	mov rax, qword ptr [rdi]
	cmp qword ptr [rax], rsi
	je .LBB_1
	ret
.LBB_1:
	add rsi, rcx
	mov qword ptr [rax], rsi
	ret