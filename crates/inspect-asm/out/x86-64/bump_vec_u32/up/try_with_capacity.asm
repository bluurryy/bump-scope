inspect_asm::bump_vec_u32::up::try_with_capacity:
	mov rax, rdi
	test rsi, rsi
	je .LBB_1
	push r14
	push rbx
	push rax
	mov rcx, rsi
	shr rcx, 61
	je .LBB_4
.LBB_9:
	mov qword ptr [rax], 0
	add rsp, 8
	pop rbx
	pop r14
	ret
.LBB_1:
	mov qword ptr [rax], 4
	xorps xmm0, xmm0
	movups xmmword ptr [rax + 8], xmm0
	mov qword ptr [rax + 24], rdx
	ret
.LBB_4:
	shl rsi, 2
	mov rdi, qword ptr [rdx]
	mov rcx, qword ptr [rdi]
	mov r9, rsi
	mov rsi, qword ptr [rdi + 8]
	add rcx, 3
	and rcx, -4
	mov rdi, rsi
	sub rdi, rcx
	cmp rdi, r9
	jb .LBB_6
	and rsi, -4
.LBB_8:
	sub rsi, rcx
	shr rsi, 2
	mov qword ptr [rax], rcx
	mov qword ptr [rax + 8], 0
	mov qword ptr [rax + 16], rsi
	mov qword ptr [rax + 24], rdx
	add rsp, 8
	pop rbx
	pop r14
	ret
.LBB_6:
	mov esi, 4
	mov rdi, rdx
	mov rbx, rdx
	mov rdx, r9
	mov r14, rax
	call bump_scope::bump_scope::BumpScope<A,_,_>::alloc_greedy_in_another_chunk
	mov rcx, rax
	mov rax, r14
	test rcx, rcx
	je .LBB_9
	mov rsi, rdx
	mov rdx, rbx
	jmp .LBB_8