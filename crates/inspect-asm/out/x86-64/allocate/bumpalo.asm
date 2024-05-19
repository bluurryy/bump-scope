inspect_asm::allocate::bumpalo:
	mov rcx, qword ptr [rdi + 16]
	mov rax, qword ptr [rcx + 32]
	sub rax, rdx
	jb .LBB_3
	mov r8, rsi
	neg r8
	and rax, r8
	cmp rax, qword ptr [rcx]
	jb .LBB_3
	mov qword ptr [rcx + 32], rax
	test rax, rax
	je .LBB_3
	ret
.LBB_3:
	push rbx
	mov rbx, rdx
	call qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]
	mov rdx, rbx
	pop rbx
	ret