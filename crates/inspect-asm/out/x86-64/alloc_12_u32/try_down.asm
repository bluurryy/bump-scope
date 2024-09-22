inspect_asm::alloc_12_u32::try_down:
	push rbx
	mov rdx, qword ptr [rdi]
	mov rcx, qword ptr [rdx]
	mov rax, rcx
	sub rax, qword ptr [rdx + 8]
	cmp rax, 47
	jbe .LBB0_0
	and rcx, -4
	mov rax, rcx
	add rax, -48
	mov qword ptr [rdx], rax
	movups xmm0, xmmword ptr [rsi]
	movups xmm1, xmmword ptr [rsi + 16]
	movups xmm2, xmmword ptr [rsi + 32]
	movups xmmword ptr [rcx - 16], xmm2
	movups xmmword ptr [rcx - 32], xmm1
	movups xmmword ptr [rax], xmm0
	je .LBB0_1
	pop rbx
	ret
.LBB0_0:
	mov rbx, rsi
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_sized_in_another_chunk
	test rax, rax
	je .LBB0_1
	movups xmm0, xmmword ptr [rbx]
	movups xmm1, xmmword ptr [rbx + 16]
	movups xmm2, xmmword ptr [rbx + 32]
	movups xmmword ptr [rax + 32], xmm2
	movups xmmword ptr [rax + 16], xmm1
	movups xmmword ptr [rax], xmm0
	pop rbx
	ret
.LBB0_1:
	xor eax, eax
	pop rbx
	ret
