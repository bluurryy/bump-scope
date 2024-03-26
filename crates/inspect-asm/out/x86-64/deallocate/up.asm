inspect_asm::deallocate::up:
	add rcx, rsi
	mov rax, qword ptr [rdi]
	cmp rcx, qword ptr [rax]
	je .LBB_1
	ret
.LBB_1:
	mov qword ptr [rax], rsi
	ret