inspect_asm::alloc_12_u32::try_down:
	push rbx
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	mov rdx, rax
	sub rdx, qword ptr [rcx + 8]
	cmp rdx, 48
	jb .LBB0_1
	and rax, -4
	add rax, -48
	je .LBB0_1
	mov qword ptr [rcx], rax
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
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_sized_in_another_chunk
	mov rsi, rbx
	test rax, rax
	jne .LBB0_0
	xor eax, eax
	pop rbx
	ret
