inspect_asm::bump_vec_u32::down::try_with_capacity:
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
	movups xmm0, xmmword ptr [rip + .L__unnamed__0]
	movups xmmword ptr [rax], xmm0
	mov qword ptr [rax + 16], 0
	mov qword ptr [rax + 24], rdx
	ret
.LBB_4:
	shl rsi, 2
	mov rdi, qword ptr [rdx]
	mov rcx, qword ptr [rdi]
	mov r9, rsi
	mov rsi, qword ptr [rdi + 8]
	and rcx, -4
	mov rdi, rcx
	sub rdi, rsi
	cmp rdi, r9
	jb .LBB_6
	add rsi, 3
	and rsi, -4
.LBB_8:
	sub rcx, rsi
	shr rcx, 2
	mov qword ptr [rax], rsi
	mov qword ptr [rax + 8], 0
	mov qword ptr [rax + 16], rcx
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
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::alloc_greedy_in_another_chunk
	mov rsi, rax
	mov rax, r14
	test rsi, rsi
	je .LBB_9
	mov rcx, rdx
	mov rdx, rbx
	jmp .LBB_8