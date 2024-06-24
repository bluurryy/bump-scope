inspect_asm::deallocate::bumpalo:
	mov rax, qword ptr [rdi + 16]
	cmp qword ptr [rax + 32], rsi
	je .LBB0_0
	ret
.LBB0_0:
	add rsi, rcx
	mov qword ptr [rax + 32], rsi
	ret
