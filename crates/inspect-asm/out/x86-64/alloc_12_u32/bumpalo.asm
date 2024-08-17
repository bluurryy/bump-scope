inspect_asm::alloc_12_u32::bumpalo:
	push rbx
	mov rcx, qword ptr [rdi + 16]
	mov rax, qword ptr [rcx + 32]
	cmp rax, 48
	jb .LBB0_1
	add rax, -48
	and rax, -4
	cmp rax, qword ptr [rcx]
	jb .LBB0_1
	mov qword ptr [rcx + 32], rax
	test rax, rax
	je .LBB0_1
.LBB0_0:
	movups xmm0, xmmword ptr [rsi]
	movups xmm1, xmmword ptr [rsi + 16]
	movups xmm2, xmmword ptr [rsi + 32]
	movups xmmword ptr [rax + 32], xmm2
	movups xmmword ptr [rax + 16], xmm1
	movups xmmword ptr [rax], xmm0
	pop rbx
	ret
.LBB0_1:
	mov rbx, rsi
	mov esi, 4
	mov edx, 48
	call qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]
	mov rsi, rbx
	test rax, rax
	jne .LBB0_0
	call qword ptr [rip + bumpalo::oom@GOTPCREL]
