inspect_asm::alloc_big::try_bumpalo:
	push rbx
	mov rax, qword ptr [rdi + 16]
	mov rbx, qword ptr [rax + 32]
	cmp rbx, 512
	jb .LBB0_1
	add rbx, -512
	and rbx, -512
	cmp rbx, qword ptr [rax]
	jb .LBB0_1
	mov qword ptr [rax + 32], rbx
	test rbx, rbx
	je .LBB0_1
.LBB0_0:
	mov edx, 512
	mov rdi, rbx
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rax, rbx
	pop rbx
	ret
.LBB0_1:
	mov rbx, rsi
	mov esi, 512
	mov edx, 512
	call qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]
	mov rsi, rbx
	mov rbx, rax
	test rax, rax
	jne .LBB0_0
	xor ebx, ebx
	mov rax, rbx
	pop rbx
	ret
