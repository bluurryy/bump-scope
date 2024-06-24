inspect_asm::deallocate::up:
	add rcx, rsi
	mov rax, qword ptr [rdi]
	cmp rcx, qword ptr [rax]
	je .LBB0_0
	ret
.LBB0_0:
	mov qword ptr [rax], rsi
	ret
