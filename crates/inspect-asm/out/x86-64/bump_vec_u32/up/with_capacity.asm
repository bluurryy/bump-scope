inspect_asm::bump_vec_u32::up::with_capacity:
	push r14
	push rbx
	push rax
	test rsi, rsi
	je .LBB_1
	mov rax, rsi
	shr rax, 61
	jne .LBB_9
	shl rsi, 2
	mov rcx, qword ptr [rdx]
	mov rax, qword ptr [rcx]
	mov rcx, qword ptr [rcx + 8]
	add rax, 3
	and rax, -4
	mov r8, rcx
	sub r8, rax
	cmp r8, rsi
	jb .LBB_5
	test rax, rax
	je .LBB_5
	and rcx, -4
.LBB_7:
	sub rcx, rax
	shr rcx, 2
	jmp .LBB_8
.LBB_1:
	mov eax, 4
	xor ecx, ecx
.LBB_8:
	mov qword ptr [rdi], rax
	mov qword ptr [rdi + 8], 0
	mov qword ptr [rdi + 16], rcx
	mov qword ptr [rdi + 24], rdx
	mov rax, rdi
	add rsp, 8
	pop rbx
	pop r14
	ret
.LBB_5:
	mov rax, rsi
	mov esi, 4
	mov r14, rdi
	mov rdi, rdx
	mov rbx, rdx
	mov rdx, rax
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::alloc_greedy_in_another_chunk
	mov rdi, r14
	mov rcx, rdx
	mov rdx, rbx
	jmp .LBB_7
.LBB_9:
	call qword ptr [rip + bump_scope::private::capacity_overflow@GOTPCREL]