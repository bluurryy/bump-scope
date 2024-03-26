inspect_asm::deallocate::bumpalo:
	mov rax, qword ptr [rdi + 16]
	cmp qword ptr [rax + 32], rsi
	je .LBB_1
	ret
.LBB_1:
	add rsi, rcx
	mov qword ptr [rax + 32], rsi
	ret