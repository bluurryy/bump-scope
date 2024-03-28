inspect_asm::alloc_iter_u32::mut_down:
	push rbp
	push r15
	push r14
	push r13
	push r12
	push rbx
	sub rsp, 40
	mov ebx, 4
	test rdx, rdx
	je .LBB_12
	mov r14, rdx
	mov rax, rdx
	shr rax, 61
	jne .LBB_17
	mov r15, rsi
	shl r14, 2
	mov rax, qword ptr [rdi]
	mov rdx, qword ptr [rax]
	mov rax, qword ptr [rax + 8]
	and rdx, -4
	mov rcx, rdx
	sub rcx, rax
	cmp rcx, r14
	jb .LBB_16
	add rax, 3
	and rax, -4
.LBB_4:
	sub rdx, rax
	shr rdx, 2
	mov qword ptr [rsp + 8], rax
	mov qword ptr [rsp + 16], 0
	mov qword ptr [rsp + 24], rdx
	mov qword ptr [rsp + 32], rdi
	xor r13d, r13d
	lea r12, [rsp + 8]
	xor edx, edx
	jmp .LBB_6
.LBB_5:
	mov dword ptr [rax + 4*rcx], ebp
	lea rdx, [rcx + 1]
	mov qword ptr [rsp + 16], rdx
	add r13, 4
	cmp r14, r13
	je .LBB_8
.LBB_6:
	mov ebp, dword ptr [r15 + r13]
	mov rcx, rdx
	cmp qword ptr [rsp + 24], rdx
	jne .LBB_5
	mov esi, 1
	mov rdi, r12
	call bump_scope::mut_bump_vec::MutBumpVec<T,A,_,_>::generic_grow_cold
	mov rax, qword ptr [rsp + 8]
	mov rcx, qword ptr [rsp + 16]
	jmp .LBB_5
.LBB_8:
	mov rax, qword ptr [rsp + 24]
	test rax, rax
	je .LBB_12
	mov rsi, qword ptr [rsp + 8]
	mov r14, qword ptr [rsp + 32]
	lea rdi, [rsi + 4*rdx]
	lea rax, [rsi + 4*rax]
	not rcx
	lea rbx, [rax + 4*rcx]
	cmp rbx, rdi
	jae .LBB_13
	mov rbx, rsi
	jmp .LBB_14
.LBB_12:
	xor edx, edx
	jmp .LBB_15
.LBB_13:
	lea rax, [4*rdx]
	mov rdi, rbx
	mov r15, rdx
	mov rdx, rax
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rdx, r15
.LBB_14:
	mov rax, qword ptr [r14]
	mov qword ptr [rax], rbx
.LBB_15:
	mov rax, rbx
	add rsp, 40
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	pop rbp
	ret
.LBB_16:
	mov esi, 4
	mov r12, rdi
	mov rdx, r14
	call bump_scope::bump_scope::BumpScope<A,_,_>::alloc_greedy_in_another_chunk
	mov rdi, r12
	jmp .LBB_4
.LBB_17:
	call qword ptr [rip + bump_scope::private::capacity_overflow@GOTPCREL]