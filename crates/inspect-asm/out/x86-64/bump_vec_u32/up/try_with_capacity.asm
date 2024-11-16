inspect_asm::bump_vec_u32::up::try_with_capacity:
	mov rax, rdi
	test rsi, rsi
	je .LBB0_1
	push r14
	push rbx
	push rax
	mov rcx, rsi
	shr rcx, 61
	je .LBB0_2
.LBB0_0:
	mov qword ptr [rax], 0
	add rsp, 8
	pop rbx
	pop r14
	ret
.LBB0_1:
	movups xmm0, xmmword ptr [rip + .L__unnamed_0]
	movups xmmword ptr [rax], xmm0
	mov qword ptr [rax + 16], 0
	mov qword ptr [rax + 24], rdx
	ret
.LBB0_2:
	shl rsi, 2
	mov rdi, qword ptr [rdx]
	mov rcx, qword ptr [rdi]
	mov r9, rsi
	mov rsi, qword ptr [rdi + 8]
	add rcx, 3
	and rcx, -4
	mov rdi, rsi
	sub rdi, rcx
	mov r8, r9
	cmp r9, rdi
	ja .LBB0_4
	test rcx, rcx
	je .LBB0_4
	and rsi, -4
.LBB0_3:
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
.LBB0_4:
	mov esi, 4
	mov rdi, rdx
	mov rbx, rdx
	mov rdx, r8
	mov r14, rax
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::prepare_allocation_in_another_chunk
	mov rcx, rax
	mov rax, r14
	test rcx, rcx
	je .LBB0_0
	mov rsi, rdx
	mov rdx, rbx
	jmp .LBB0_3
